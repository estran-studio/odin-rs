//! Wavetable data for Odin 2 synthesizer
//!
//! Auto-generated from WavetableData.cpp
//! DO NOT EDIT MANUALLY

// Données numériques : certaines valeurs approchent des constantes mathématiques par hasard.
#![allow(clippy::approx_constant, clippy::large_const_arrays)]

use crate::constants::{SUBTABLES_PER_WAVETABLE, WAVETABLE_LENGTH};

mod names;
pub use names::WAVETABLE_NAMES;
pub mod lfo_data;

mod wt_000;
mod wt_001;
mod wt_002;
mod wt_003;
mod wt_004;
mod wt_005;
mod wt_006;
mod wt_007;
mod wt_008;
mod wt_009;
mod wt_010;
mod wt_011;
mod wt_012;
mod wt_013;
mod wt_014;
mod wt_015;
mod wt_016;
mod wt_017;
mod wt_018;
mod wt_019;
mod wt_020;
mod wt_021;
mod wt_022;
mod wt_023;
mod wt_024;
mod wt_025;
mod wt_026;
mod wt_027;
mod wt_028;
mod wt_029;
mod wt_030;
mod wt_031;
mod wt_032;
mod wt_033;
mod wt_034;
mod wt_035;
mod wt_036;
mod wt_037;
mod wt_038;
mod wt_039;
mod wt_040;
mod wt_041;
mod wt_042;
mod wt_043;
mod wt_044;
mod wt_045;
mod wt_046;
mod wt_047;
mod wt_048;
mod wt_049;
mod wt_050;
mod wt_051;
mod wt_052;
mod wt_053;
mod wt_054;
mod wt_055;
mod wt_056;
mod wt_057;
mod wt_058;
mod wt_059;
mod wt_060;
mod wt_061;
mod wt_062;
mod wt_063;
mod wt_064;
mod wt_065;
mod wt_066;
mod wt_067;
mod wt_068;
mod wt_069;
mod wt_070;
mod wt_071;
mod wt_072;
mod wt_073;
mod wt_074;
mod wt_075;
mod wt_076;
mod wt_077;
mod wt_078;
mod wt_079;
mod wt_080;
mod wt_081;
mod wt_082;
mod wt_083;
mod wt_084;
mod wt_085;
mod wt_086;
mod wt_087;
mod wt_088;
mod wt_089;
mod wt_090;
mod wt_091;
mod wt_092;
mod wt_093;
mod wt_094;
mod wt_095;
mod wt_096;
mod wt_097;
mod wt_098;
mod wt_099;
mod wt_100;
mod wt_101;
mod wt_102;
mod wt_103;
mod wt_104;
mod wt_105;
mod wt_106;
mod wt_107;
mod wt_108;
mod wt_109;
mod wt_110;
mod wt_111;
mod wt_112;
mod wt_113;
mod wt_114;
mod wt_115;
mod wt_116;
mod wt_117;
mod wt_118;
mod wt_119;
mod wt_120;
mod wt_121;
mod wt_122;
mod wt_123;
mod wt_124;
mod wt_125;
mod wt_126;
mod wt_127;
mod wt_128;
mod wt_129;
mod wt_130;
mod wt_131;
mod wt_132;
mod wt_133;
mod wt_134;
mod wt_135;
mod wt_136;
mod wt_137;
mod wt_138;
mod wt_139;
mod wt_140;
mod wt_141;
mod wt_142;
mod wt_143;
mod wt_144;
mod wt_145;
mod wt_146;
mod wt_147;
mod wt_148;
mod wt_149;
mod wt_150;
mod wt_151;
mod wt_152;
mod wt_153;
mod wt_154;
mod wt_155;
mod wt_156;
mod wt_157;
mod wt_158;
mod wt_159;

/// Get a wavetable by index
/// Returns a reference to the subtables array [33][512]
pub fn get_wavetable(index: usize) -> &'static [[f32; WAVETABLE_LENGTH]; SUBTABLES_PER_WAVETABLE] {
    match index {
        0 => &wt_000::DATA,
        1 => &wt_001::DATA,
        2 => &wt_002::DATA,
        3 => &wt_003::DATA,
        4 => &wt_004::DATA,
        5 => &wt_005::DATA,
        6 => &wt_006::DATA,
        7 => &wt_007::DATA,
        8 => &wt_008::DATA,
        9 => &wt_009::DATA,
        10 => &wt_010::DATA,
        11 => &wt_011::DATA,
        12 => &wt_012::DATA,
        13 => &wt_013::DATA,
        14 => &wt_014::DATA,
        15 => &wt_015::DATA,
        16 => &wt_016::DATA,
        17 => &wt_017::DATA,
        18 => &wt_018::DATA,
        19 => &wt_019::DATA,
        20 => &wt_020::DATA,
        21 => &wt_021::DATA,
        22 => &wt_022::DATA,
        23 => &wt_023::DATA,
        24 => &wt_024::DATA,
        25 => &wt_025::DATA,
        26 => &wt_026::DATA,
        27 => &wt_027::DATA,
        28 => &wt_028::DATA,
        29 => &wt_029::DATA,
        30 => &wt_030::DATA,
        31 => &wt_031::DATA,
        32 => &wt_032::DATA,
        33 => &wt_033::DATA,
        34 => &wt_034::DATA,
        35 => &wt_035::DATA,
        36 => &wt_036::DATA,
        37 => &wt_037::DATA,
        38 => &wt_038::DATA,
        39 => &wt_039::DATA,
        40 => &wt_040::DATA,
        41 => &wt_041::DATA,
        42 => &wt_042::DATA,
        43 => &wt_043::DATA,
        44 => &wt_044::DATA,
        45 => &wt_045::DATA,
        46 => &wt_046::DATA,
        47 => &wt_047::DATA,
        48 => &wt_048::DATA,
        49 => &wt_049::DATA,
        50 => &wt_050::DATA,
        51 => &wt_051::DATA,
        52 => &wt_052::DATA,
        53 => &wt_053::DATA,
        54 => &wt_054::DATA,
        55 => &wt_055::DATA,
        56 => &wt_056::DATA,
        57 => &wt_057::DATA,
        58 => &wt_058::DATA,
        59 => &wt_059::DATA,
        60 => &wt_060::DATA,
        61 => &wt_061::DATA,
        62 => &wt_062::DATA,
        63 => &wt_063::DATA,
        64 => &wt_064::DATA,
        65 => &wt_065::DATA,
        66 => &wt_066::DATA,
        67 => &wt_067::DATA,
        68 => &wt_068::DATA,
        69 => &wt_069::DATA,
        70 => &wt_070::DATA,
        71 => &wt_071::DATA,
        72 => &wt_072::DATA,
        73 => &wt_073::DATA,
        74 => &wt_074::DATA,
        75 => &wt_075::DATA,
        76 => &wt_076::DATA,
        77 => &wt_077::DATA,
        78 => &wt_078::DATA,
        79 => &wt_079::DATA,
        80 => &wt_080::DATA,
        81 => &wt_081::DATA,
        82 => &wt_082::DATA,
        83 => &wt_083::DATA,
        84 => &wt_084::DATA,
        85 => &wt_085::DATA,
        86 => &wt_086::DATA,
        87 => &wt_087::DATA,
        88 => &wt_088::DATA,
        89 => &wt_089::DATA,
        90 => &wt_090::DATA,
        91 => &wt_091::DATA,
        92 => &wt_092::DATA,
        93 => &wt_093::DATA,
        94 => &wt_094::DATA,
        95 => &wt_095::DATA,
        96 => &wt_096::DATA,
        97 => &wt_097::DATA,
        98 => &wt_098::DATA,
        99 => &wt_099::DATA,
        100 => &wt_100::DATA,
        101 => &wt_101::DATA,
        102 => &wt_102::DATA,
        103 => &wt_103::DATA,
        104 => &wt_104::DATA,
        105 => &wt_105::DATA,
        106 => &wt_106::DATA,
        107 => &wt_107::DATA,
        108 => &wt_108::DATA,
        109 => &wt_109::DATA,
        110 => &wt_110::DATA,
        111 => &wt_111::DATA,
        112 => &wt_112::DATA,
        113 => &wt_113::DATA,
        114 => &wt_114::DATA,
        115 => &wt_115::DATA,
        116 => &wt_116::DATA,
        117 => &wt_117::DATA,
        118 => &wt_118::DATA,
        119 => &wt_119::DATA,
        120 => &wt_120::DATA,
        121 => &wt_121::DATA,
        122 => &wt_122::DATA,
        123 => &wt_123::DATA,
        124 => &wt_124::DATA,
        125 => &wt_125::DATA,
        126 => &wt_126::DATA,
        127 => &wt_127::DATA,
        128 => &wt_128::DATA,
        129 => &wt_129::DATA,
        130 => &wt_130::DATA,
        131 => &wt_131::DATA,
        132 => &wt_132::DATA,
        133 => &wt_133::DATA,
        134 => &wt_134::DATA,
        135 => &wt_135::DATA,
        136 => &wt_136::DATA,
        137 => &wt_137::DATA,
        138 => &wt_138::DATA,
        139 => &wt_139::DATA,
        140 => &wt_140::DATA,
        141 => &wt_141::DATA,
        142 => &wt_142::DATA,
        143 => &wt_143::DATA,
        144 => &wt_144::DATA,
        145 => &wt_145::DATA,
        146 => &wt_146::DATA,
        147 => &wt_147::DATA,
        148 => &wt_148::DATA,
        149 => &wt_149::DATA,
        150 => &wt_150::DATA,
        151 => &wt_151::DATA,
        152 => &wt_152::DATA,
        153 => &wt_153::DATA,
        154 => &wt_154::DATA,
        155 => &wt_155::DATA,
        156 => &wt_156::DATA,
        157 => &wt_157::DATA,
        158 => &wt_158::DATA,
        159 => &wt_159::DATA,
        _ => &wt_000::DATA, // Default to first wavetable
    }
}

/// Get a specific subtable from a wavetable
pub fn get_subtable(wavetable: usize, subtable: usize) -> &'static [f32; WAVETABLE_LENGTH] {
    &get_wavetable(wavetable)[subtable.min(SUBTABLES_PER_WAVETABLE - 1)]
}
