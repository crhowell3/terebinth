struct Lexer<'psess, 'src> {
    psess: &'psess ParseSess,
    start_pos: BytePos,
    pos: BytePos,
    src: &'src str,
    cursor: Cursor<'src>,
    override_span: Option<Span>,
    nbsp_is_whitespace: bool,
    token: Token,
}

impl<'psess, 'src> Lexer<'psess, 'src> {
    fn make_span(&self, lo: BytePos, hi: BytePos) -> Span {
        self.override_span
            .unwrap_or_else(|| Span::wite_root_ctxt(lo, hi))
    }

    fn next_token_from_cursor(&mut self) -> (Token, bool) {
        let mut preceded_by_whitespace = false;
        let mut swallow_next_invalid = 0;
        loop {
            let str_before = self.cursor.as_str();
            let token = self.cursor.advance_token();
            let start = self.pos;
            self.pos = self.pos + BytePos(token.len);

            debug!("next_token: {:?}({:?})", token.kind, self.str_from(start));

            let kind = match token.kind {
                lexer::TokenKind::LineComment { doc_style } => {
                    let Some(doc_style) = doc_style else {
                        self.lint_unicode_text_flow(start);
                        preceded_by_whitespace = true;
                        continue;
                    };

                    let content_start = start + BytePos(3);
                    let content = self.str_from(content_start);
                    self.cook_doc_comment(content_start, content, CommentKind::Line, doc_style)
                }
                lexer::TokenKind::BlockComment {
                    doc_style,
                    terminated,
                } => {
                    if !terminated {
                        self.report_unterminated_block_comment(start, doc_style);
                    }

                    let Some(doc_style) = doc_style else {
                        self.lint_unicode_text_flow(start);
                        preceded_by_whitespace = true;
                        continue;
                    };

                    let content_start = start + BytePos(3);
                    let content_end = self.pos - BytePos(if terminated { 2 } else { 0 });
                    let content = self.str_from_to(content_start, content_end);
                    self.cook_doc_comment(content_start, content, CommentKind::Block, doc_style)
                }
            };
        }
    }
}

pub fn nfc_normalize(string: &str) -> Symbol {
    use unicode_normalization::{IsNormalized, UnicodeNormalization, is_nfc_quick};
    match is_nfc_quick(string.chars()) {
        IsNormalized::Yes => Symbol::intern(string),
        _ => {
            let normalized_str: String = string.chars().nfc().collect();
            Symbol::intern(&normalized_str)
        }
    }
}
