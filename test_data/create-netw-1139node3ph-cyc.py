#!/usr/bin/env python3

import epyjson as epj

abc = ["A", "B", "C"]
n_variants = 15


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


netw_orig = epj.EJson.read_from_file("netw-78node3ph-cyc.json")
netw_orig_raw = epj.EJson.read_from_file("netw-78node3ph-cyc.json").raw_ejson

netws = [netw_orig.clone() for _ in range(n_variants)]
for (i, netw) in enumerate(netws):
    rename_dict = {c["id"]: variant(c["id"], i) for c in netw.components()}
    netw.rename_to(rename_dict)

for netw in netws[1:10]:
    trim(netw, "node_1-")

for netw in netws[10:]:
    trim(netw, "node_4-")

all_comps = [c for netw in netws for c in netw.raw_ejson["components"]]

netw = epj.EJson(netw.properties | {"components": all_comps})

for i in range(1, 10):
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

print(f"Number of nodes = {len(list(netw.components('Node')))}")
print(f"Number of loads = {len(list(netw.components('Load')))}")
print(f"Number of lines = {len(list(netw.components('Line')))}")
print(f"Number of transformers = {len(list(netw.components('Transformer')))}")

netw.write_to_file("netw-1139node3ph-cyc.json")
