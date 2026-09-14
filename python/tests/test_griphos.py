import griphos as gr

def test_network():
    netw = gr.parse_ejson_file("../test_data/feeder.json")
    print(f"\nNetwork:\n{netw}")
    gr.solve_network(netw, "nodal_rect")
