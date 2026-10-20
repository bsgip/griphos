import griphos as gr
from griphos import bt

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
    assert abs((b.vs[0] - complex(11000.0, 0.0))) < 1e-6


def test_generic_element():
    yc = [
            ((0, 0), gr.C_ONE),
            ((1, 1), gr.C_ONE),
            ((1, 1), complex(0.5, 0.25)),
    ]
    e: gr.GenericElement = gr.GenericElement(
        "my_ge",
        [
            bt("bus_1", gr.Phase.A),
            bt("bus_1", gr.Phase.B),
            bt("bus_1", gr.Phase.C),
        ],
        [(0, 1), (1, 2), (2, 0)],
        y_const=yc
    )
    assert e.y_const == yc
    assert e.i_const == []
    assert e.s_const == []
    assert e.uid == "my_ge"


def test_network():
    netw = gr.parse_ejson_file(f"{TEST_DATA_DIR}/netw-i3l3s3.json")
    results = gr.solve_network(netw, "nodal_rect")
    assert results.statuses[0].status.flags & gr.SUCCESS != 0
    assert results.statuses[0].status.flags & gr.FAILURE == 0
