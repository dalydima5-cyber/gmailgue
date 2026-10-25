use std::env;
use std::io::{self, BufWriter, Write};

const DOMAIN: &[u8] = b"@gmail.com\n";
const BUF_CAPACITY: usize = 1024 * 1024; // 1 MB I/O buffer

// Precomputed 2-digit ASCII lookup table ("00" to "99") for zero-division number formatting
const DIGITS_2: &[u8; 200] = b"\
    0001020304050607080910111213141516171819\
    2021222324252627282930313233343536373839\
    4041424344454647484950515253545556575859\
    6061626364656667686970717273747576777879\
    8081828384858687888990919293949596979899";

/// Stamps a 2-digit number (00..99) directly into a 2-byte slice
#[inline(always)]
fn stamp_2d(dst: &mut [u8], n: usize) {
    let idx = n * 2;
    dst[0] = DIGITS_2[idx];
    dst[1] = DIGITS_2[idx + 1];
}

/// Stamps a 4-digit number (0000..9999) directly into a 4-byte slice using the lookup table
#[inline(always)]
fn stamp_4d(dst: &mut [u8], high_00_99: usize, low_00_99: usize) {
    let h_idx = high_00_99 * 2;
    let l_idx = low_00_99 * 2;
    dst[0] = DIGITS_2[h_idx];
    dst[1] = DIGITS_2[h_idx + 1];
    dst[2] = DIGITS_2[l_idx];
    dst[3] = DIGITS_2[l_idx + 1];
}

/// Builds 24 distinct name permutations (initials, slices, dots, reversals)
fn build_bases(f: &[u8], l: &[u8]) -> Vec<Vec<u8>> {
    let mut bases = Vec::with_capacity(28);

    let f1 = &f[0..1];
    let l1 = &l[0..1];
    let f2 = if f.len() >= 2 { &f[0..2] } else { f };
    let l2 = if l.len() >= 2 { &l[0..2] } else { l };
    let f3 = if f.len() >= 3 { &f[0..3] } else { f };
    let l3 = if l.len() >= 3 { &l[0..3] } else { l };

    // Core combinations
    bases.push([f, l].concat());                 // 1. jhondoe
    bases.push([f, b".", l].concat());           // 2. jhon.doe
    bases.push([l, f].concat());                 // 3. doejhon
    bases.push([l, b".", f].concat());           // 4. doe.jhon

    // Initials + full
    bases.push([f1, l].concat());                // 5. jdoe
    bases.push([f1, b".", l].concat());          // 6. j.doe
    bases.push([f, l1].concat());                // 7. jhond
    bases.push([f, b".", l1].concat());          // 8. jhon.d
    bases.push([l, f1].concat());                // 9. doej
    bases.push([l, b".", f1].concat());          // 10. doe.j
    bases.push([l1, f].concat());                // 11. djhon
    bases.push([l1, b".", f].concat());          // 12. d.jhon

    // 2-letter and 3-letter slices (very common in real usernames)
    bases.push([f3, l].concat());                // 13. jhodoe
    bases.push([f, l3].concat());                // 14. jhondo
    bases.push([f3, l3].concat());               // 15. jhodoe (shortened)
    bases.push([f2, l].concat());                // 16. jhdoe
    bases.push([f, l2].concat());                // 17. jhondo
    bases.push([l3, f3].concat());               // 18. doejho

    // Doubled initials & standalone
    bases.push([f1, f, l].concat());             // 19. jjhondoe
    bases.push([f, l, l1].concat());             // 20. jhondoed
    bases.push([f1, l1].concat());               // 21. jd (used with 4+ digits)
    bases.push([f, f].concat());                 // 22. jhonjhon
    bases.push(f.to_vec());                      // 23. jhon
    bases.push(l.to_vec());                      // 24. doe

    // Deduplicate in case input names are already 2-3 letters long
    bases.sort();
    bases.dedup();
    bases
}

/// Rapidly stamps 0..9, 00..99, 000..999, and 0000..9999 onto a base slice
fn emit_all_numbers_suffix<W: Write>(out: &mut W, base: &[u8]) -> io::Result<()> {
    let blen = base.len();
    let mut buf = [0u8; 128];
    buf[..blen].copy_from_slice(base);

    // 1. Base alone (if >= 6 chars)
    if blen >= 6 {
        buf[blen..blen + DOMAIN.len()].copy_from_slice(DOMAIN);
        out.write_all(&buf[..blen + DOMAIN.len()])?;
    }

    // 2. 1-digit (0..9)
    if blen + 1 >= 6 {
        let total = blen + 1 + DOMAIN.len();
        buf[blen + 1..total].copy_from_slice(DOMAIN);
        for d in b'0'..=b'9' {
            buf[blen] = d;
            out.write_all(&buf[..total])?;
        }
    }

    // 3. 2-digit (00..99, covers both 01..09 and 10..99)
    if blen + 2 >= 6 {
        let total = blen + 2 + DOMAIN.len();
        buf[blen + 2..total].copy_from_slice(DOMAIN);
        for n in 0..100 {
            stamp_2d(&mut buf[blen..blen + 2], n);
            out.write_all(&buf[..total])?;
        }
    }

    // 4. 3-digit (000..999)
    if blen + 3 >= 6 {
        let total = blen + 3 + DOMAIN.len();
        buf[blen + 3..total].copy_from_slice(DOMAIN);
        for d1 in b'0'..=b'9' {
            buf[blen] = d1;
            for n in 0..100 {
                stamp_2d(&mut buf[blen + 1..blen + 3], n);
                out.write_all(&buf[..total])?;
            }
        }
    }

    // 5. 4-digit (0000..9999 -> covers 3278, 5556, 5729, all years, all PINs)
    if blen + 4 >= 6 {
        let total = blen + 4 + DOMAIN.len();
        buf[blen + 4..total].copy_from_slice(DOMAIN);
        for high in 0..100 {
            let h_idx = high * 2;
            buf[blen] = DIGITS_2[h_idx];
            buf[blen + 1] = DIGITS_2[h_idx + 1];
            for low in 0..100 {
                let l_idx = low * 2;
                buf[blen + 2] = DIGITS_2[l_idx];
                buf[blen + 3] = DIGITS_2[l_idx + 1];
                out.write_all(&buf[..total])?;
            }
        }
    }

    Ok(())
}

/// Rapidly stamps 0000..9999 in the MIDDLE between two slices (e.g. jhon3278doe)
fn emit_middle_numbers<W: Write>(out: &mut W, left: &[u8], right: &[u8]) -> io::Result<()> {
    let llen = left.len();
    let rlen = right.len();
    let mut buf = [0u8; 128];
    buf[..llen].copy_from_slice(left);

    // 2-digit middle (00..99)
    if llen + 2 + rlen >= 6 {
        let r_start = llen + 2;
        let d_start = r_start + rlen;
        let total = d_start + DOMAIN.len();
        buf[r_start..d_start].copy_from_slice(right);
        buf[d_start..total].copy_from_slice(DOMAIN);

        for n in 0..100 {
            stamp_2d(&mut buf[llen..llen + 2], n);
            out.write_all(&buf[..total])?;
        }
    }

    // 4-digit middle (0000..9999 -> e.g., jhon3278doe, doe5729jhon)
    if llen + 4 + rlen >= 6 {
        let r_start = llen + 4;
        let d_start = r_start + rlen;
        let total = d_start + DOMAIN.len();
        buf[r_start..d_start].copy_from_slice(right);
        buf[d_start..total].copy_from_slice(DOMAIN);

        for high in 0..100 {
            for low in 0..100 {
                stamp_4d(&mut buf[llen..llen + 4], high, low);
                out.write_all(&buf[..total])?;
            }
        }
    }

    Ok(())
}

/// Rapidly stamps 0000..9999 as a PREFIX before a base slice (e.g. 3278jhondoe)
fn emit_prefix_numbers<W: Write>(out: &mut W, base: &[u8]) -> io::Result<()> {
    let blen = base.len();
    let mut buf = [0u8; 128];

    // 4-digit prefix (0000..9999 + base)
    if 4 + blen >= 6 {
        let d_start = 4 + blen;
        let total = d_start + DOMAIN.len();
        buf[4..d_start].copy_from_slice(base);
        buf[d_start..total].copy_from_slice(DOMAIN);

        for high in 0..100 {
            for low in 0..100 {
                stamp_4d(&mut buf[0..4], high, low);
                out.write_all(&buf[..total])?;
            }
        }
    }
    Ok(())
}

/// Generates high-priority combinations for a specific year (e.g. -2008)
fn emit_year_specials<W: Write>(out: &mut W, bases: &[Vec<u8>], f: &[u8], l: &[u8], yr: &[u8]) -> io::Result<()> {
    let short_yr = if yr.len() == 4 { &yr[2..4] } else { yr };
    let mut line = Vec::with_capacity(128);

    for base in bases {
        // 1. base + DDMM + 2008 & base + DDMM + 08 & base + MMDD + 2008
        for month in 1..=12 {
            for day in 1..=31 {
                let mut ddmm = [0u8; 4];
                stamp_2d(&mut ddmm[0..2], day);
                stamp_2d(&mut ddmm[2..4], month);

                // base + DDMM + 2008
                line.clear();
                line.extend_from_slice(base);
                line.extend_from_slice(&ddmm);
                line.extend_from_slice(yr);
                line.extend_from_slice(DOMAIN);
                out.write_all(&line)?;

                // base + DDMM + 08
                if short_yr != yr {
                    line.clear();
                    line.extend_from_slice(base);
                    line.extend_from_slice(&ddmm);
                    line.extend_from_slice(short_yr);
                    line.extend_from_slice(DOMAIN);
                    out.write_all(&line)?;
                }

                // base + 2008 + DDMM
                line.clear();
                line.extend_from_slice(base);
                line.extend_from_slice(yr);
                line.extend_from_slice(&ddmm);
                line.extend_from_slice(DOMAIN);
                out.write_all(&line)?;
            }
        }

        // 2. base + 2008 + (00..99) -> e.g. jhondoe200801, jhondoe200899
        for n in 0..100 {
            let mut d2 = [0u8; 2];
            stamp_2d(&mut d2, n);
            line.clear();
            line.extend_from_slice(base);
            line.extend_from_slice(yr);
            line.extend_from_slice(&d2);
            line.extend_from_slice(DOMAIN);
            out.write_all(&line)?;
        }
    }

    // 3. Year in the middle + suffix numbers: jhon2008doe01..99
    for n in 0..100 {
        let mut d2 = [0u8; 2];
        stamp_2d(&mut d2, n);
        line.clear();
        line.extend_from_slice(f);
        line.extend_from_slice(yr);
        line.extend_from_slice(l);
        line.extend_from_slice(&d2);
        line.extend_from_slice(DOMAIN);
        out.write_all(&line)?;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().skip(1).collect();

    let mut names: Vec<String> = Vec::new();
    let mut target_year: Option<String> = None;

    for arg in args {
        if let Some(stripped) = arg.strip_prefix('-') {
            if !stripped.is_empty() && stripped.chars().all(|c| c.is_ascii_digit()) {
                target_year = Some(stripped.to_string());
            }
        } else {
            // Strip any accidental spaces or symbols and lowercase
            let cleaned: String = arg
                .chars()
                .filter(|c| c.is_ascii_alphabetic())
                .map(|c| c.to_ascii_lowercase())
                .collect();
            if !cleaned.is_empty() {
                names.push(cleaned);
            }
        }
    }

    if names.len() < 2 {
        eprintln!("Usage: gmailgue <first_name> <last_name> [-YEAR]");
        eprintln!("Example: gmailgue jhon doe -2008 > emails.txt");
        std::process::exit(1);
    }

    let f = names[0].as_bytes();
    let l = names[1].as_bytes();

    // 1 MB Buffered Writer locked directly to Linux stdout
    let stdout = io::stdout();
    let mut out = BufWriter::with_capacity(BUF_CAPACITY, stdout.lock());

    let bases = build_bases(f, l);

    // Phase 1: If -YEAR (like -2008) is passed, emit targeted year + date/number combos first
    if let Some(ref yr) = target_year {
        emit_year_specials(&mut out, &bases, f, l, yr.as_bytes())?;
    }

    // Phase 2: Full 1-digit, 2-digit, 3-digit, and 4-digit (0000..9999) sweep across ALL ~24 bases
    // Covers 3278, 5556, 5729 on jhondoe, doejhon, jdoe, jhond, jhodoe, jhon, doe, etc.
    for base in &bases {
        emit_all_numbers_suffix(&mut out, base)?;
    }

    // Phase 3: Middle 2-digit & 4-digit numbers (0000..9999)
    // Covers jhon3278doe, doe5556jhon, jhon.5729.doe, doe.3278.jhon
    emit_middle_numbers(&mut out, f, l)?;
    emit_middle_numbers(&mut out, l, f)?;
    let f_dot = [f, b"."].concat();
    let dot_l = [b".", l].concat();
    let l_dot = [l, b"."].concat();
    let dot_f = [b".", f].concat();
    emit_middle_numbers(&mut out, &f_dot, &dot_l)?;
    emit_middle_numbers(&mut out, &l_dot, &dot_f)?;

    // Phase 4: Prefix 4-digit numbers (0000..9999) before primary combinations
    // Covers 3278jhondoe, 5556doejhon, 5729jhon.doe
    let primary_prefixes = [
        [f, l].concat(),
        [l, f].concat(),
        [f, b".", l].concat(),
        [l, b".", f].concat(),
    ];
    for pbase in &primary_prefixes {
        emit_prefix_numbers(&mut out, pbase)?;
    }

    out.flush()?;
    Ok(())
}