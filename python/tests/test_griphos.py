import griphos as gr

from pathlib import Path

TEST_DATA_DIR = Path(__file__).resolve().parents[2] / "test_data"

def test_network():
    netw = gr.parse_ejson_file(f"{TEST_DATA_DIR}/netw-i3l3s3.json")
    print(f"\nNetwork:\n{netw}")
    gr.solve_network(netw, "nodal_rect")
