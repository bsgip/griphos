import griphos as gr
from griphos import nd_id

from pathlib import Path

TEST_DATA_DIR = Path(__file__).resolve().parents[2] / "test_data"

def test_complex():
    c: complex[float] = complex(1.0, 0.0)
    assert c == gr.C_ONE
    assert type(c) is type(gr.C_ONE)


def test_phase():
    pa: gr.Phase = gr.Phase.A
    pb: gr.Phase = gr.Phase.B
    assert pa != pb
    assert type(pa) is type(pb)


def test_bus():
    b = gr.Bus("my_bus", [gr.Phase.A, gr.Phase.B, gr.Phase.C], 11000.0)
    assert b.uid == "my_bus"
    assert b.phases == [gr.Phase.A, gr.Phase.B, gr.Phase.C]
    assert b.v_base == 11000.0
    assert len(b.vs) == 3
    assert abs(b.vs[0] - complex(11000.0, 0.0)) < 1e-6


def test_generic_element():
    yc = [
            ((0, 0), gr.C_ONE),
            ((1, 1), gr.C_ONE),
            ((1, 1), complex(0.5, 0.25)),
    ]
    e: gr.GenericElement = gr.GenericElement(
        "my_ge",
        [
            nd_id("bus_1", gr.Phase.A),
            nd_id("bus_1", gr.Phase.B),
            nd_id("bus_1", gr.Phase.C),
        ],
        [(0, 1), (1, 2), (2, 0)],
        y_const=yc
    )
    assert e.y_const == yc
    assert e.i_const == []
    assert e.s_const == []
    assert e.uid == "my_ge"


def test_transformer():
    taps_data: gr.TransformerTapsData = gr.TransformerTapsData(
        0, 0.01, -3, 3
    )

    tx: gr.Transformer = gr.Transformer.new_with_vector_group(
        "tx",
        [
            nd_id("bus_p", gr.Phase.A),
            nd_id("bus_p", gr.Phase.B),
            nd_id("bus_p", gr.Phase.C),
            nd_id("bus_s", gr.Phase.A),
            nd_id("bus_s", gr.Phase.B),
            nd_id("bus_s", gr.Phase.C),
            nd_id("bus_s", gr.Phase.N),
        ],
        3,
        [11000+0j, 230+0j],
        taps_data,
        [0+0j, 0+0.01j],
        "dyn11"
    )

    taps = tx.taps
    assert tx.turns_ratio(0, 1, 0) == (11000+0j)/(230+0j)
    assert tx.turns_ratio(0, 1, 1) == (11000+0j)/(230+0j)
    assert tx.turns(0, 0) == 11000+0j
    assert tx.turns(0, 1) == 11000+0j
    assert tx.turns(1, 0) == 230+0j
    assert tx.turns(1, 1) == 230+0j
    taps.set(1, 1)
    assert tx.turns(0, 0) == 11000+0j
    assert tx.turns(0, 1) == 1.01 * 11000+0j
    assert tx.turns(1, 0) == 230+0j
    assert tx.turns(1, 1) == 230+0j

def test_network():
    netw = gr.parse_ejson_file(f"{TEST_DATA_DIR}/netw-i3l3s3.json")
    results = gr.solve_network(netw, "nodal_rect")
    assert results.statuses[0].status.flags & gr.SUCCESS != 0
    assert results.statuses[0].status.flags & gr.FAILURE == 0
