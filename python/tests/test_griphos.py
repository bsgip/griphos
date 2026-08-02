import griphos as gr

def test_network():
    netw = gr.Network()
    bus = gr.Bus("b1", [gr.Phase.A], 11.0)
    gr.solve_network(netw, "nodal_rect")
