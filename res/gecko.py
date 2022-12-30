"""Renders GDB view of the Gecko CPU core"""


def main(buffer, waveform, vcd_header, cursor):
    """Main function"""
    buffer.set_cell(0, 0, "#")
    buffer.set_cell(0, 1, "#")
    buffer.set_cell(1, 0, "#")
    buffer.set_cell(1, 1, "f")

    # signal = vcd_header.get_variable("TOP.clk")
    # if signal is None:
    #     raise Exception("Signal not found!")

    timestamp_index = waveform.search_timestamp(cursor)
    if timestamp_index is None:
        raise Exception("Timestamp index not found!")

    for i, c in enumerate(str(timestamp_index)):
        buffer.set_cell(i, 2, c)

    signal = vcd_header.get_variable(
        "TOP.gecko_nano_wrapper.inst.core.gecko_decode_inst.regfile.register_file_inst.xilinx_distributed_ram_inst.data[1]"
    )
    if signal is None:
        raise Exception("Signal not found!")

    result = waveform.search_value(signal.get_idcode(), timestamp_index)
    if result is None:
        raise Exception("Result not found!")

    for i, c in enumerate(str(result.get_timestamp_index())):
        buffer.set_cell(i, 4, c)

    vector = result.get_vector()
    if vector is None:
        raise Exception("Vector not found!")

    value = vector.get_value()

    for i, c in enumerate(hex(value)):
        buffer.set_cell(i, 5, c)

    return buffer
