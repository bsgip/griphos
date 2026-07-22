from griphos import ejson_phase_map_abcng, Network, parse_ejson

def parse_ejson_file(fname: str, phase_map: dict = ejson_phase_map_abcng()) -> Network:
    """
    Parse an e-json file (with the given filename) into a Network.
    """
    with open(fname) as f:
        netw = parse_ejson(f, phase_map)

    return netw
