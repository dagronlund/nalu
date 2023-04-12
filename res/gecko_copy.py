"""Nalu config script"""

from nalu import new_group, new_vector, new_signal, new_spacer, SignalRadix

# Fix some issues with imports?
import os
import sys

sys.path.append(os.path.dirname(os.path.realpath(__file__)))

import gecko_interactive


### BEGIN NALU GENERATED CODE ###
# fmt: off
def nalu_config(vcd_header):
    """Nalu generated waveform config"""
    return [
        new_signal("TOP.exit_code[8]", SignalRadix.Hexadecimal, False, None),
    ]
# fmt: on
### END NALU GENERATED CODE ###


def user_config(vcd_header):
    """User generated waveform config"""
    # Add user-defined signal config here
    return [new_signal("TOP.rst", SignalRadix.Hexadecimal, False)]


def interactive(buffer, waveform, vcd_header, cursor):
    """Custom waveform visualization/interaction"""
    return gecko_interactive.interactive(buffer, waveform, vcd_header, cursor)
