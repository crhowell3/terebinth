pub fn analyze_source_file(src: &str) -> (Vec<RelativeBytePos>, Vec<MultiByteChar>) {
    let mut lines = vec![RelativeBytePos::from_u32(0)];
    let mut multi_byte_chars = vec![];

    analyze_source_file_dispatch(src, &mut lines, &mut multi_byte_chars);

    if let Some(&last_line_start) = lines.last() {
        let source_files_end = RelativeBytePos::from_usize(src.len());
        assert!(source_files_end >= last_line_start);
        if last_line_start == source_files_end {
            lines.pop();
        }
    }

    (lines, multi_byte_chars)
}

cfg_match! {
    any(target_arch = "x86", target_arch = "x86_64") => {
        fn analyze_source_file_dispatch(
            src: &str,
            lines: &mut Vec<RelativeBytePos>,
            multi_byte_chars: &mut Vec<MultiByteChar>,
        ) {
            if is_x86_feature_detected("sse2") {
                unsafe {
                    analyze_source_file_sse2(src, lines, multi_byte_chars);
                }
            } else {
                analyze_source_file_generic(
                    src, src.len(),
                    RelativeBytePos::from_u32(0),
                    lines,
                    multi_byte_chars,
                );
            }
        }

        #[target_feature(enable = "sse2")]
        unsafe fn analyze_source_file_sse2(
            src: &str,
            lines: &mut Vec<RelativeBytePos>,
            multi_byte_chars: &mut Vec<MultiByteChar>,
        ) {
            #[cfg(target_arch = "x86")]
            use std::arch::x86::*;
            #[cfg(target_arch = "x86_64")]
            use std::arch::x86_64::*;

            const CHUNK_SIZE: usize = 16;

            let (chunks, tail) = src.as_bytes().as_chunks::<CHUNK_SIZE>();

            let mut intra_chunk_offset = 0;

            for (chunk_index, chunk) in chunks.iter().enumerate() {
                let chunk = unsafe { _mm_loadu_si128(chunk.as_ptr() as *const __m128i)};
                let multibyte_test = _mm_cmplt_epi8(chunk, _mm_set1_epi8(0));
                let multibyte_mask = _mm_movemask_epi8(multibyte_test);

                if multibyte_mask == 0 {
                    assert!(intra_chunk_offset == 0);

                    let newlines_test = _mm_cmpeq_epi8(chunk, _mm_set1_epi8(b'\n' as i8));
                    let mut newlines_mask = _mm_movemask_epi8(newlines_test);

                    let output_offset = RelativeBytePos::from_usize(chunk_index * CHUNK_SIZE + 1);

                    while newlines_mask != 0 {
                        let index = newlines_mask.trailing_zeros();

                        lines.push(RelativeBytePos(index) + output_offset);

                        newlines_mask &= newlines_mask - 1;
                    }
                } else {
                    let scan_start = chunk_index * CHUNK_SIZE + intra_chunk_offset;
                    intra_chunk_offset = analyze_source_file_generic(
                        &src[scan_start..],
                        CHUNK_SIZE - intra_chunk_offset,
                        RelativeBytePos::from_usize(scan_start),
                        lines,
                        multi_byte_chars,
                    );
                }
            }

            let tail_start = src.len() - tail.len() + intra_chunk_offset;
            if tail_start < src.len() {
                analyze_source_file_generic(
                    &src[tail_start..],
                    src.len() - tail_start,
                    RelativeBytePos::from_usize(tail_start),
                    lines,
                    multi_byte_chars,
                );
            }
        }
    }
    _ => {
        fn analyze_source_file_dispatch(
            src: &str,
            lines: &mut Vec<RelativeBytePos>,
            multi_byte_chars: &mut Vec<MultiByteChar>,
        ) {
            analyze_source_file_generic(
                src, src.len(), RelativeBytePos::from_u32(0), lines,
                multi_byte_chars,
            );
        }
    }
}

fn analyze_source_file_generic(
    src: &str,
    scan_len: usize,
    output_offset: RelativeBytePos,
    lines: &mut Vec<RelativeBytePos>,
    multi_byte_chars: &mut Vec<MultiByteChar>,
) -> usize {
    assert!(src.len() >= scan_len);
    let mut i = 0;
    let src_bytes = src.as_bytes();

    while i < scan_len {
        let byte = unsafe { *src_bytes.get_unchecked(i) };

        let mut char_len = 1;

        if byte == b'\n' {
            let pos = RelativeBytePos::from_usize(i) + output_offset;
            lines.push(pos + RelativeBytePos(1));
        } else if byte >= 128 {
            let c = src[i..].chars().next().unwrap();
            char_len = c.len_utf8();

            let pos = RelativeBytePos::from_usize(i) + output_offset;
            assert!((2..=4).contains(&char_len));
            let mbc = MultiByteChar {
                pos,
                bytes: char_len as u8,
            };
            multi_byte_chars.push(mbc);
        }

        i += char_len;
    }

    i - scan_len
}
