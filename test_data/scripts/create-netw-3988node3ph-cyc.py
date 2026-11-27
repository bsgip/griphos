#!/usr/bin/env python3

import random

import epyjson as epj

abc = ["A", "B", "C"]
# As for netw-1139node3ph-cyc, plus extra copies trimmed at node_1 and pasted onto the head (~77 nodes each).
# The extras are appended after the original 15 variants so that variant indices referenced below are unchanged.
n_extra_variants = 37
n_variants = 15 + n_extra_variants
node_1_variants = list(range(1, 10)) + list(range(15, n_variants))


def variant(s, variant):
    return f"{s}-{variant}"


def rename_comp(comp, variant):
    comp["id"] = variant(comp["id"], variant)
    if "cons" in comp:
        for con in comp["cons"]:
            con["node"] = variant(con["node"], variant)


def trim(netw: epj.EJson, stop_node_prefix):
    inf_id = next(netw.components("Infeeder"))["id"]
    netw.trim(inf_id, lambda netw, comp: comp["id"].startswith(stop_node_prefix))


def mod_new(comp, variant):
    comp["id"] += variant
    if "cons" in comp:
        for con in comp["cons"]:
            con["node"] = con["node"] + variant


def add_connector(netw, nd_1, nd_2):
    connector_id = f"connector-{nd_1}-{nd_2}"
    netw.add_comp({
        "id": connector_id,
        "type": "Connector",
        "switch_state": "closed",
    })
    netw.connect(connector_id, nd_1, None, {"phs": abc})
    netw.connect(connector_id, nd_2, None, {"phs": abc})


def sum_cplxs(cs: list[list[float]]):
    return [[sum(ci) for ci in zip(*cs)]]

netw_orig = epj.EJson.read_from_file("netw-78node3ph-cyc.json")
netw_orig_raw = epj.EJson.read_from_file("netw-78node3ph-cyc.json").raw_ejson

netws = [netw_orig.clone() for _ in range(n_variants)]
for (i, netw) in enumerate(netws):
    rename_dict = {c["id"]: variant(c["id"], i) for c in netw.components()}
    netw.rename_to(rename_dict)

for i in node_1_variants:
    trim(netws[i], "node_1-")

for netw in netws[10:15]:
    trim(netw, "node_4-")

all_comps = [c for netw in netws for c in netw.raw_ejson["components"]]

netw = epj.EJson(netw.properties | {"components": all_comps})

for i in node_1_variants:
    add_connector(netw, variant("node_1", i), variant("node_3", 0))

add_connector(netw, variant("node_4", 10), variant("node_11", 0))
add_connector(netw, variant("node_4", 11), variant("node_58", 1))
add_connector(netw, variant("node_14", 12), variant("node_23", 1))
add_connector(netw, variant("node_35", 12), variant("node_5", 1))
add_connector(netw, variant("node_4", 13), variant("node_4", 3))
add_connector(netw, variant("node_7", 14), variant("node_6", 0))
add_connector(netw, variant("node_39", 10), variant("node_28", 3))

epj.coalesce_connectors(netw)

epj.scale_loads(netw, 0.4)

# Vary vector group of one particular transformer.
netw.component(variant("transformer_1", 1))["vector_group"] = "dyn5"

# Make one of the LV networks single phase.
nd_3_6_id = variant("node_3", 6)

tx_1_6_id = variant("transformer_1", 6)
tx_1_6 = netw.component(tx_1_6_id)
tx_1_6["n_winding_pairs"] = 1
tx_1_6["taps"] = [0]
tx_1_6_cons = list(netw.connections_from(tx_1_6_id))

sp = netw.dfs(variant("node_4", 6), stop_cb=lambda netw, c: c["id"] == nd_3_6_id)[0]
for comp_id in sp:
    comp = netw.component(comp_id)
    if comp["type"] == "Node":
        comp["phs"] = ["B"]
    elif comp["type"] == "Load":
        comp["s"] = sum_cplxs(comp["s"])

    for con in netw.connections_from(comp_id):
        con.con["phs"] = ["B"]

tx_1_6_cons[0].con["phs"] = ["A", "B"]

# Make a fraction of loads, their nodes, and service lines single phase.
random.seed(1234)
lds = list(netw.components("Load"))
lds_sp = random.sample(lds, len(lds) // 2)
for ld in lds_sp:
    for ld_con in netw.connections_from(ld["id"]):
        ld_nd_id = ld_con.cid_1
        ld_nd = netw.component(ld_nd_id)
        service_ids = [con.cid_1 for con in netw.connections_from(ld_nd_id) if con.cid_1 != ld["id"]]
        if not (len(ld_nd["phs"]) == 3 and len(service_ids) == 1):
            continue
        service_id = service_ids[0]

        service = netw.component(service_id)

        if service["type"] != "Line":
            continue

        ph = random.choice(["A", "B", "C"])
        new_phs = [ph]
        ld_con.con["phs"] = new_phs
        ld["s"] = sum_cplxs(ld["s"])
        ld_nd["phs"] = new_phs
        for service_con in netw.connections_from(service_id):
            service_con.con["phs"] = new_phs

print(f"Number of nodes = {len(list(netw.components('Node')))}")
print(f"Number of loads = {len(list(netw.components('Load')))}")
print(f"Number of lines = {len(list(netw.components('Line')))}")
print(f"Number of transformers = {len(list(netw.components('Transformer')))}")

netw.write_to_file("netw-huge.json")
