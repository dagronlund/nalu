"""Renders GDB view of the Gecko CPU core"""

from nalu import WaveformSearchMode

from riscvmodel.insn import *


class MachineDecodeError(Exception):
    def __init__(self, word):
        self.word = word

    def __str__(self):
        return "Invalid instruction word: {:08x}".format(self.word)


def decode(word: int, variant: Variant = RV32I):
    if word & 0x3 != 3:
        # compact
        for icls in get_insns(cls=InstructionCType):
            if icls._match(word):
                i = icls()
                i.decode(word)
                return i
        raise MachineDecodeError(word)
    opcode = word & 0x7F
    for icls in get_insns(variant=variant):
        if icls.field_opcode.value == opcode and icls.match(word):
            i = icls()
            i.decode(word)
            return i
    raise MachineDecodeError(word)


def get_reg_name(reg_num):
    reg_name = [
        "x0",
        "ra",
        "sp",
        "gp",
        "tp",
        "t0",
        "t1",
        "t2",
        "s0",
        "s1",
        "a0",
        "a1",
        "a2",
        "a3",
        "a4",
        "a5",
        "a6",
        "a7",
        "s2",
        "s3",
        "s4",
        "s5",
        "s6",
        "s7",
        "s8",
        "s9",
        "s10",
        "s11",
        "t3",
        "t4",
        "t5",
        "t6",
    ]
    return reg_name[reg_num]


def get_reg_path(reg_num):
    return f"TOP.gecko_nano_wrapper.inst.core.gecko_decode_inst.regfile.register_file_inst.xilinx_distributed_ram_inst.data[{reg_num}]"


def get_reg_status_front_path(reg_num):
    return f"TOP.gecko_nano_wrapper.inst.core.gecko_decode_inst.regfile.register_status_front_inst.xilinx_distributed_ram_inst.data[{reg_num}]"


def get_reg_status_rear_path(reg_num):
    return f"TOP.gecko_nano_wrapper.inst.core.gecko_decode_inst.regfile.register_status_rear_inst.xilinx_distributed_ram_inst.data[{reg_num}]"


def get_pc_path():
    return "TOP.gecko_nano_wrapper.inst.core.gecko_fetch_inst.pc"


def get_mem_path(addr):
    return f"TOP.gecko_nano_wrapper.inst.mem.gen_xilinx.xilinx_block_ram_double_inst.data[{addr}]"


def get_reg_info(waveform, vcd_header, buffer, timestamp_index, line=0):
    for reg in range(32):
        # Find register signal
        signal = vcd_header.get_variable(get_reg_path(reg))
        result = waveform.search_value(signal.get_idcode(), timestamp_index)
        reg_value = result.get_vector().get_value()
        # Find front register status signal
        signal = vcd_header.get_variable(get_reg_status_front_path(reg))
        result = waveform.search_value(signal.get_idcode(), timestamp_index)
        reg_status_front_value = result.get_vector().get_value()
        # Find rear register status signal
        signal = vcd_header.get_variable(get_reg_status_rear_path(reg))
        result = waveform.search_value(signal.get_idcode(), timestamp_index)
        reg_status_rear_value = result.get_vector().get_value()
        # Format register message
        reg_num = f"x{reg}".ljust(3)
        reg_value = "0x{:08x}".format(reg_value)
        reg_status = (reg_status_front_value - reg_status_rear_value) & 0x7
        info = f"{reg_num} ({get_reg_name(reg).ljust(3)}) {reg_value} ({reg_status})"
        # Find screen offsets
        header = "--Registers"
        for x in range(buffer.get_width()):
            if x < len(header):
                buffer.set_cell(x, line, header[x])
            else:
                buffer.set_cell(x, line, "-")
        x = (reg // 8) * 32
        y = reg % 8
        for i, char in enumerate(info):
            buffer.set_cell(x + i, y + line + 1, char)


def get_instruction_info(waveform, vcd_header, buffer, timestamp_index, line=0):
    # Find pc signal
    signal = vcd_header.get_variable(get_pc_path())
    result = waveform.search_value(signal.get_idcode(), timestamp_index)
    pc_value = result.get_vector().get_value()

    # Find screen offsets
    header = "--Instructions"
    for x in range(buffer.get_width()):
        if x < len(header):
            buffer.set_cell(x, line, header[x])
        else:
            buffer.set_cell(x, line, "-")

    for i, offset in enumerate(range(-3, 4)):
        pc_offset = pc_value + offset
        if pc_offset < 0:
            continue

        # Find memory signal
        signal = vcd_header.get_variable(get_mem_path(pc_offset))
        result = waveform.search_value(signal.get_idcode(), timestamp_index)
        mem_value = result.get_vector().get_value()

        header = ">" if offset == 0 else "-"
        try:
            inst = decode(mem_value)
        except:
            inst = "<unknown>"
        mem_value = "0x{:08x}".format(mem_value)
        pc_offset = "0x{:08x}".format(pc_offset)
        info = f"{header}{pc_offset} ({mem_value}) {str(inst)}"
        for j, char in enumerate(info):
            buffer.set_cell(j, line + i + 1, char)


def main(buffer, waveform, vcd_header, cursor):
    """Main function"""

    timestamp_index = waveform.search_timestamp(cursor, int(WaveformSearchMode.Before))
    if timestamp_index is None:
        raise Exception("Timestamp index not found!")

    for i, c in enumerate(
        f"Timestamp:       {str(waveform.get_timestamp(timestamp_index))}"
    ):
        buffer.set_cell(i, 1, c)
    for i, c in enumerate(f"Timestamp Index: {str(timestamp_index)}"):
        buffer.set_cell(i, 2, c)

    get_reg_info(waveform, vcd_header, buffer, timestamp_index, line=8)

    get_instruction_info(waveform, vcd_header, buffer, timestamp_index, line=20)

    return buffer
