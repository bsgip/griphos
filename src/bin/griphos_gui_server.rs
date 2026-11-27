use std::collections::HashMap;
use std::panic::{self, AssertUnwindSafe};
use std::time::Instant;

use clap::Parser;
use log::{info, warn};
use serde_json::{Value, json};
use tiny_http::{Header, Method, Response, Server};

use griphos::*;

#[derive(Parser)]
struct Args {
    #[arg(long, default_value = "localhost")]
    host: String,
    #[arg(short, long, default_value_t = 34568)]
    port: u16,
    #[arg(short, long, default_value = "nodal_rect")]
    solver: String,
    /// Print requests and responses (long bodies are truncated).
    #[arg(short, long)]
    verbose: bool,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Branch,
    Gen,
    Load,
}

fn kind(e: &Element) -> Option<Kind> {
    if e.is::<Line>() || e.is::<Transformer>() || e.is::<Connector>() {
        Some(Kind::Branch)
    } else if e.is::<Source>() {
        Some(Kind::Gen)
    } else if e.is::<Load>() {
        Some(Kind::Load)
    } else {
        None
    }
}

// Neutral (Phase::N) nodes are hidden from clients, and buses with only neutral nodes are omitted.
fn is_neutral(p: Phase) -> bool {
    matches!(p, Phase::N)
}

fn shown_nodes(b: &Bus) -> impl Iterator<Item = &Node> {
    b.nodes.iter().filter(|nd| !is_neutral(nd.phase()))
}

fn bus_v_rms_pu(b: &Bus) -> f64 {
    let v_sq: Vec<f64> = shown_nodes(b).map(|nd| nd.v().norm_sqr()).collect();
    if v_sq.is_empty() { 0.0 } else { (v_sq.iter().sum::<f64>() / v_sq.len() as f64).sqrt() / b.v_base }
}

fn bus_json(b: &Bus) -> Value {
    let phs: Vec<String> = shown_nodes(b).map(|nd| nd.phase().to_string()).collect();
    json!({
        "id": b.uid,
        "phs": phs,
        "isInService": true,
        "latlong": {"lat": 0.0, "long": 0.0},
        "vBase": b.v_base,
        "vRmsPu": bus_v_rms_pu(b),
        "properties": {"type": "Bus"},
    })
}

/// Connections of an element, grouped by bus in order of first appearance, omitting neutrals.
fn cons_json(e: &ElementBorrowed) -> Vec<Value> {
    let mut cons: Vec<(&str, Vec<String>)> = Vec::new();
    for nd in e.node_ids().iter().filter(|nd| !is_neutral(nd.p)) {
        match cons.iter_mut().find(|(b, _)| *b == nd.b) {
            Some((_, phs)) => phs.push(nd.p.to_string()),
            None => cons.push((&nd.b, vec![nd.p.to_string()])),
        }
    }
    cons.into_iter().map(|(b, phs)| json!({"bus": b, "phs": phs})).collect()
}

/// Total complex power consumed by an element, in MW.
fn s_tot(e: &ElementBorrowed) -> c64 {
    e.branches().map(|br| br.v() * br.i().conj()).sum()
}

/// Total real power in MW: generated for gens, consumed otherwise.
fn element_p_tot(e: &Element, kind: Kind) -> f64 {
    let p = s_tot(&e.borrow()).re;
    if kind == Kind::Gen { -p } else { p }
}

fn element_json(e: &Element, kind: Kind) -> Value {
    let p_tot = element_p_tot(e, kind);
    let e = e.borrow();
    let mut j = json!({
        "id": e.uid(),
        "isInService": e.is_in_service(),
        "properties": {"type": e.element_type()},
    });
    let cons = cons_json(&e);
    if kind == Kind::Branch {
        j["cons"] = cons.into();
        j["branchType"] = e.element_type().to_lowercase().into();
    } else {
        j["con"] = cons.into_iter().next().unwrap_or(Value::Null);
        j["pTot"] = p_tot.into();
    }
    j
}

fn is_shown(b: &Bus) -> bool {
    shown_nodes(b).next().is_some()
}

fn buses(netw: &Network) -> impl Iterator<Item = Shared<Bus>> + '_ {
    netw.buses().filter(|b| is_shown(&b.borrow()))
}

fn elements_of_kind(netw: &Network, k: Kind) -> impl Iterator<Item = Element> + '_ {
    netw.elements().filter(move |e| kind(e) == Some(k))
}

fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = (b[i] == b'%' && i + 2 < b.len())
            .then(|| std::str::from_utf8(&b[i + 1..i + 3]).ok().and_then(|h| u8::from_str_radix(h, 16).ok()))
            .flatten();
        match (b[i], hex) {
            (_, Some(x)) => {
                out.push(x);
                i += 3;
            }
            (b'+', _) => {
                out.push(b' ');
                i += 1;
            }
            (c, _) => {
                out.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn query_params(url: &str) -> HashMap<String, String> {
    let query = url.split_once('?').map_or("", |(_, q)| q);
    query
        .split('&')
        .filter(|kv| !kv.is_empty())
        .map(|kv| {
            let (k, v) = kv.split_once('=').unwrap_or((kv, ""));
            (url_decode(k), url_decode(v))
        })
        .collect()
}

struct GuiServer {
    solver: String,
    netw: Option<Network>,
    solvers: Option<NetworkSolvers>,
}

impl GuiServer {
    fn handle_get(&self, params: &HashMap<String, String>) -> (u16, Option<Value>) {
        let Some(netw) = &self.netw else { return (404, None) };
        let view = params.get("view").map_or("", |s| s.as_str());
        let id = params.get("id").map_or("", |s| s.as_str());
        let element_view = |k: Kind| match netw.element(id).filter(|e| kind(e) == Some(k)) {
            Some(e) => (200, Some(element_json(&e, k))),
            None => (404, None),
        };
        match view {
            "network" => {
                let elems = |k: Kind| elements_of_kind(netw, k).map(|e| element_json(&e, k)).collect::<Vec<_>>();
                let buses: Vec<Value> = buses(netw).map(|b| bus_json(&b.borrow())).collect();
                let reply = json!({
                    "branches": elems(Kind::Branch),
                    "buses": buses,
                    "gens": elems(Kind::Gen),
                    "loads": elems(Kind::Load),
                });
                (200, Some(reply))
            }
            "state" => {
                let powers = |k: Kind| elements_of_kind(netw, k).map(|e| element_p_tot(&e, k)).collect::<Vec<_>>();
                let bus_voltages: Vec<f64> = buses(netw).map(|b| bus_v_rms_pu(&b.borrow())).collect();
                let reply = json!({
                    "busVoltages": bus_voltages,
                    "genPowers": powers(Kind::Gen),
                    "loadPowers": powers(Kind::Load),
                });
                (200, Some(reply))
            }
            "bus" => match netw.bus(id).filter(|b| is_shown(&b.borrow())) {
                Some(b) => (200, Some(bus_json(&b.borrow()))),
                None => (404, None),
            },
            "branch" => element_view(Kind::Branch),
            "gen" => element_view(Kind::Gen),
            "load" => element_view(Kind::Load),
            _ => (400, None),
        }
    }

    fn handle_post(&mut self, params: &HashMap<String, String>, body: &str) -> (u16, Option<Value>) {
        match params.get("action").map_or("", |s| s.as_str()) {
            "load_network" => {
                self.solvers = None;
                self.netw = parse_ejson(body.as_bytes(), &ejson_phase_map_abcng())
                    .inspect_err(|err| warn!("Error loading network: {err}"))
                    .ok();
                (if self.netw.is_some() { 200 } else { 500 }, None)
            }
            "solve_network" => {
                let Some(netw) = &mut self.netw else { return (404, None) };
                let t_start = Instant::now();
                match solve_network(netw, &self.solver, self.solvers.take()) {
                    Ok(result) => {
                        let solve_time_ms = t_start.elapsed().as_millis() as u64;
                        let ok = result.statuses.iter().all(|x| x.status.flags.contains(SolverStatusFlags::SUCCESS));
                        self.solvers = Some(result.solvers);
                        (200, Some(json!({"ok": ok, "solve_time_ms": solve_time_ms})))
                    }
                    Err(err) => {
                        warn!("Error solving network: {err:?}");
                        (500, None)
                    }
                }
            }
            "toggle" => {
                let Some(netw) = &self.netw else { return (404, None) };
                let id = params.get("id").map_or("", |s| s.as_str());
                match netw.element(id).filter(|e| kind(e).is_some()) {
                    Some(e) => {
                        let mut e = e.borrow_mut();
                        let is_in_service = e.is_in_service();
                        e.set_is_in_service(!is_in_service);
                        (200, None)
                    }
                    None => (404, None),
                }
            }
            _ => (400, None),
        }
    }

    fn handle(&mut self, method: &Method, url: &str, body: &str) -> (u16, Option<Value>) {
        let params = query_params(url);
        match method {
            Method::Get => self.handle_get(&params),
            Method::Post => self.handle_post(&params, body),
            _ => (405, None),
        }
    }
}

const VERBOSE_MAX_CHARS: usize = 2000;

fn truncated(s: &str) -> String {
    match s.char_indices().nth(VERBOSE_MAX_CHARS) {
        Some((i, _)) => format!("{}\n... [truncated, {} bytes total]", &s[..i], s.len()),
        None => s.to_string(),
    }
}

fn main() {
    configure_logging();
    let args = Args::parse();

    let addr = format!("{}:{}", args.host, args.port);
    let server = Server::http(&addr).unwrap_or_else(|err| panic!("Could not listen on {addr}: {err}"));
    println!("Listening for requests on http://{addr}");

    let mut gui_server = GuiServer { solver: args.solver, netw: None, solvers: None };
    for mut request in server.incoming_requests() {
        info!("{} {}", request.method(), request.url());
        let mut body = String::new();
        let (status, reply) = match request.as_reader().read_to_string(&mut body) {
            Ok(_) => {
                if args.verbose {
                    println!("--> {} {}", request.method(), request.url());
                    if !body.is_empty() {
                        println!("{}", truncated(&body));
                    }
                }
                // Abort just this request on panic. The network may be partially modified, so discard the solvers.
                let (method, url) = (request.method(), request.url());
                panic::catch_unwind(AssertUnwindSafe(|| gui_server.handle(method, url, &body))).unwrap_or_else(|_| {
                    warn!("Request {method} {url} panicked; aborted.");
                    gui_server.solvers = None;
                    (500, None)
                })
            }
            Err(err) => {
                warn!("Error reading request body: {err}");
                (400, None)
            }
        };
        let reply = reply.map(|r| serde_json::to_string_pretty(&r).unwrap());
        if args.verbose {
            println!("<-- {status}");
            if let Some(reply) = &reply {
                println!("{}", truncated(reply));
            }
        }
        let mut response = match reply {
            Some(reply) => Response::from_string(reply)
                .with_header(Header::from_bytes("Content-Type", "application/json").unwrap()),
            None => Response::from_string(""),
        }
        .with_status_code(status);
        response.add_header(Header::from_bytes("Access-Control-Allow-Origin", "*").unwrap());
        if let Err(err) = request.respond(response) {
            warn!("Error sending response: {err}");
        }
    }
}
