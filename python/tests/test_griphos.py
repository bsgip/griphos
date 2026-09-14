import griphos as gr

def test_network():
    netw = gr.parse_ejson_file("../test_data/netw_a.json")
    print(netw)
    gr.solve_network(netw, "nodal_rect")
