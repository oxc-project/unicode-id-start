use crate::output::Output;
use crate::parse::Properties;
use crate::CHUNK;

const HEAD: &str = "\
const T: bool = true;
const F: bool = false;

#[repr(C, align(8))]
pub(crate) struct Align8<T>(pub(crate) T);
#[repr(C, align(64))]
pub(crate) struct Align64<T>(pub(crate) T);
";

pub fn output(
    properties: &Properties,
    index_start: &[u8],
    index_continue: &[u8],
    halfdense: &[u8],
    id_start_high: &[(u32, u32)],
    id_continue_high: &[(u32, u32)],
) -> Output {
    let mut out = Output::new();
    writeln!(out, "{}", HEAD);

    writeln!(
        out,
        "pub(crate) static ASCII_START: Align64<[bool; 128]> = Align64([",
    );
    for i in 0u8..4 {
        write!(out, "   ");
        for j in 0..32 {
            let ch = (i * 32 + j) as char;
            let is_id_start = properties.is_id_start(ch);
            write!(out, " {},", if is_id_start { 'T' } else { 'F' });
        }
        writeln!(out);
    }
    writeln!(out, "]);");
    writeln!(out);

    writeln!(
        out,
        "pub(crate) static ASCII_CONTINUE: Align64<[bool; 128]> = Align64([",
    );
    for i in 0u8..4 {
        write!(out, "   ");
        for j in 0..32 {
            let ch = (i * 32 + j) as char;
            let is_id_continue = properties.is_id_continue(ch);
            write!(out, " {},", if is_id_continue { 'T' } else { 'F' });
        }
        writeln!(out);
    }
    writeln!(out, "]);");
    writeln!(out);

    writeln!(out, "pub(crate) const CHUNK: usize = {};", CHUNK);
    writeln!(out);

    // Codepoints living above the end of each trie, kept as explicit ranges so the trie index
    // does not have to span the large run of empty entries below them. See `split_high`.
    for (name, ranges) in [
        ("ID_START_HIGH", id_start_high),
        ("ID_CONTINUE_HIGH", id_continue_high),
    ] {
        writeln!(
            out,
            "pub(crate) static {name}: [(u32, u32); {}] = [",
            ranges.len(),
        );
        for (lo, hi) in ranges {
            writeln!(out, "    (0x{:X}, 0x{:X}),", lo, hi);
        }
        writeln!(out, "];");
        writeln!(out);
    }

    writeln!(
        out,
        "pub(crate) static TRIE_START: Align8<[u8; {}]> = Align8([",
        index_start.len(),
    );
    for line in index_start.chunks(16) {
        write!(out, "   ");
        for byte in line {
            write!(out, " 0x{:02X},", byte);
        }
        writeln!(out);
    }
    writeln!(out, "]);");
    writeln!(out);

    writeln!(
        out,
        "pub(crate) static TRIE_CONTINUE: Align8<[u8; {}]> = Align8([",
        index_continue.len(),
    );
    for line in index_continue.chunks(16) {
        write!(out, "   ");
        for byte in line {
            write!(out, " 0x{:02X},", byte);
        }
        writeln!(out);
    }
    writeln!(out, "]);");
    writeln!(out);

    writeln!(
        out,
        "pub(crate) static LEAF: Align64<[u8; {}]> = Align64([",
        halfdense.len(),
    );
    for line in halfdense.chunks(16) {
        write!(out, "   ");
        for byte in line {
            write!(out, " 0x{:02X},", byte);
        }
        writeln!(out);
    }
    writeln!(out, "]);");

    out
}
