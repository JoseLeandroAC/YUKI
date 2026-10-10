// ============================================================================
// Secure HTML to Plain Text Extraction (Data != Instruction)
// ============================================================================

/// Extrai texto útil limpo e sanitizado de uma resposta HTML bruta.
///
/// REGRAS DE HIGIENE E SEGURANÇA:
/// 1. Remove integralmente blocos executáveis e de estilo (`<script>`, `<style>`, `<noscript>`).
/// 2. Remove tags de mídia, formulários e iframes (`<iframe>`, `<form>`, `<input>`, `<svg>`, `<canvas>`).
/// 3. Extrai `<title>` como metadado de título da página.
/// 4. Converte blocos estruturais (`<p>`, `<div>`, `<h1>`-`<h6>`, `<li>`, `<br>`, `<tr>`) em quebras de linha limpas.
/// 5. Decodifica entidades HTML canônicas sem expansão recursiva (imunidade contra entity bombs).
/// 6. Normaliza espaços múltiplos e sequências excessivas de quebras de linha.
/// 7. Trunca a saída conforme `max_chars` com sinalização de `truncated`.
/// 8. Invariante constitucional: O texto extraído é DADO BRUTO NÃO CONFIÁVEL (`Data != Instruction`).
pub fn extract_text_from_html(html: &str, max_chars: usize) -> (String, Option<String>, bool) {
    let title = extract_title(html);

    // 1. Remover blocos perigosos e não-textuais (e seu conteúdo interno)
    let cleaned_html = strip_dangerous_blocks(html);

    // 2. Converter blocos estruturais em quebras de linha
    let with_breaks = convert_structural_tags(&cleaned_html);

    // 3. Remover todas as tags HTML restantes
    let raw_text = strip_all_tags(&with_breaks);

    // 4. Decodificar entidades HTML padrão
    let decoded = decode_html_entities(&raw_text);

    // 5. Normalizar espaços e quebras de linha
    let normalized = normalize_whitespace(&decoded);

    // 6. Aplicar truncamento delimitado
    let (final_text, truncated) = if normalized.chars().count() > max_chars {
        let truncated_str: String = normalized.chars().take(max_chars).collect();
        (truncated_str, true)
    } else {
        (normalized, false)
    };

    (final_text, title, truncated)
}

/// Extrai o conteúdo da tag `<title>` se presente.
fn extract_title(html: &str) -> Option<String> {
    let lower = html.to_lowercase();
    let start_tag = "<title";
    let end_tag = "</title>";

    let start_idx = lower.find(start_tag)?;
    let after_tag = &html[start_idx..];
    let close_bracket = after_tag.find('>')?;
    let content_start = start_idx + close_bracket + 1;

    let end_idx = html[content_start..]
        .to_lowercase()
        .find(end_tag)
        .map(|idx| content_start + idx)?;

    let raw_title = &html[content_start..end_idx];
    let title_clean = strip_all_tags(raw_title);
    let title_decoded = decode_html_entities(&title_clean);
    let title_trimmed = title_decoded.trim();

    if title_trimmed.is_empty() {
        None
    } else {
        let truncated: String = title_trimmed.chars().take(200).collect();
        Some(truncated)
    }
}

/// Remove blocos completos de script, style, form, iframe, etc.
fn strip_dangerous_blocks(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    let dangerous_tags = [
        "script", "style", "noscript", "svg", "canvas", "iframe", "object", "embed", "form",
    ];

    while i < len {
        // Verificar início de comentário HTML <!-- ... -->
        if i + 3 < len
            && chars[i] == '<'
            && chars[i + 1] == '!'
            && chars[i + 2] == '-'
            && chars[i + 3] == '-'
        {
            i += 4;
            while i + 2 < len && !(chars[i] == '-' && chars[i + 1] == '-' && chars[i + 2] == '>') {
                i += 1;
            }
            i += 3; // pular -->
            continue;
        }

        // Verificar início de tag perigosa <tag ...> ... </tag>
        if chars[i] == '<' {
            let mut tag_name = String::new();
            let mut j = i + 1;
            while j < len && (chars[j].is_alphanumeric() || chars[j] == '_') {
                tag_name.push(chars[j].to_ascii_lowercase());
                j += 1;
            }

            if dangerous_tags.contains(&tag_name.as_str()) {
                let closing_tag = format!("</{}>", tag_name);
                let closing_chars: Vec<char> = closing_tag.chars().collect();

                // Avançar até o fechamento da tag de abertura
                while j < len && chars[j] != '>' {
                    j += 1;
                }
                if j < len {
                    j += 1; // pular '>'
                }

                // Avançar até a tag de fechamento </tag>
                let mut closed = false;
                while j < len {
                    if chars[j] == '<' {
                        let matches_closing = (0..closing_chars.len()).all(|k| {
                            j + k < len && chars[j + k].to_ascii_lowercase() == closing_chars[k]
                        });
                        if matches_closing {
                            j += closing_chars.len();
                            closed = true;
                            break;
                        }
                    }
                    j += 1;
                }

                if closed {
                    i = j;
                    output.push(' ');
                    continue;
                }
            }
        }

        output.push(chars[i]);
        i += 1;
    }

    output
}

/// Converte tags estruturais de bloco em novas linhas.
fn convert_structural_tags(input: &str) -> String {
    let mut res = String::with_capacity(input.len());
    let mut i = 0;
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();

    while i < len {
        if chars[i] == '<' {
            let mut tag = String::new();
            let mut j = i + 1;
            while j < len && chars[j] != '>' {
                tag.push(chars[j].to_ascii_lowercase());
                j += 1;
            }
            if j < len {
                j += 1; // pular '>'
            }

            let tag_trimmed = tag.trim();
            if tag_trimmed.starts_with("br")
                || tag_trimmed.starts_with("/p")
                || tag_trimmed.starts_with("/div")
                || tag_trimmed.starts_with("/h1")
                || tag_trimmed.starts_with("/h2")
                || tag_trimmed.starts_with("/h3")
                || tag_trimmed.starts_with("/h4")
                || tag_trimmed.starts_with("/h5")
                || tag_trimmed.starts_with("/h6")
                || tag_trimmed.starts_with("/li")
                || tag_trimmed.starts_with("/tr")
                || tag_trimmed.starts_with("/section")
                || tag_trimmed.starts_with("/article")
                || tag_trimmed.starts_with("/header")
                || tag_trimmed.starts_with("/footer")
            {
                res.push('\n');
            } else {
                res.push('<');
                res.push_str(&tag);
                res.push('>');
            }
            i = j;
            continue;
        }

        res.push(chars[i]);
        i += 1;
    }

    res
}

/// Remove todas as tags HTML remanescentes.
fn strip_all_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;

    for c in input.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
            out.push(' ');
        } else if !in_tag {
            out.push(c);
        }
    }

    out
}

/// Decodifica entidades HTML conhecidas sem recursão.
fn decode_html_entities(input: &str) -> String {
    input
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&copy;", "©")
        .replace("&reg;", "®")
        .replace("&trade;", "™")
        .replace("&mdash;", "—")
        .replace("&ndash;", "–")
}

/// Normaliza espaços múltiplos e quebras de linha excessivas.
fn normalize_whitespace(input: &str) -> String {
    let mut lines = Vec::new();

    for line in input.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            // Colapsar espaços múltiplos dentro da linha
            let collapsed: Vec<&str> = trimmed.split_whitespace().collect();
            lines.push(collapsed.join(" "));
        }
    }

    lines.join("\n\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_basic_html() {
        let html = r#"
            <!DOCTYPE html>
            <html>
            <head><title>Título do Teste</title></head>
            <body>
                <h1>Cabeçalho Principal</h1>
                <p>Este é um parágrafo com <strong>negrito</strong> e &amp; entidades.</p>
                <script>alert('malicious');</script>
                <style>body { color: red; }</style>
                <p>Segundo parágrafo.</p>
            </body>
            </html>
        "#;

        let (text, title, truncated) = extract_text_from_html(html, 1000);
        assert_eq!(title.as_deref(), Some("Título do Teste"));
        assert!(!truncated);
        assert!(text.contains("Cabeçalho Principal"));
        assert!(text.contains("Este é um parágrafo com negrito e & entidades."));
        assert!(text.contains("Segundo parágrafo."));
        assert!(!text.contains("alert"));
        assert!(!text.contains("color: red"));
    }

    #[test]
    fn test_truncation() {
        let html = "<p>Texto longo que deve ser truncado</p>";
        let (text, _, truncated) = extract_text_from_html(html, 10);
        assert!(truncated);
        assert_eq!(text.chars().count(), 10);
    }
}
