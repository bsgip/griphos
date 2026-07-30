import griphos as gr

def test_network():
    netw = gr.Network()
    bus = gr.Bus("b1", [gr.Phase.A], 11.0)
    print("BUS")
    print(bus)
