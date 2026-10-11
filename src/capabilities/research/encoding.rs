// ============================================================================
// Yuki Personal AI Platform — Research v1
// Content-Encoding & Bounded Streaming Decompression (ADR-020)
// ============================================================================

use crate::contracts::errors::YukiError;
use std::io::Read;

/// Codificações HTTP Content-Encoding suportadas de forma segura e delimitada.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportedContentEncoding {
    Identity,
    Gzip,
    Deflate,
    Brotli,
}

impl SupportedContentEncoding {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Gzip => "gzip",
            Self::Deflate => "deflate",
            Self::Brotli => "br",
        }
    }

    /// Analisa o cabeçalho HTTP Content-Encoding, aplicando política fail-closed para valores desconhecidos.
    pub fn parse(header_value: Option<&str>) -> Result<Self, YukiError> {
        let val = match header_value {
            Some(v) => v.trim().to_lowercase(),
            None => return Ok(Self::Identity),
        };

        if val.is_empty() || val == "identity" || val == "none" {
            Ok(Self::Identity)
        } else if val == "gzip" || val == "x-gzip" {
            Ok(Self::Gzip)
        } else if val == "deflate" {
            Ok(Self::Deflate)
        } else if val == "br" {
            Ok(Self::Brotli)
        } else {
            Err(YukiError::ExecutionFailed(format!(
                "Content-Encoding '{}' não suportado pelo provedor de leitura governada (suportados: gzip, deflate, br, identity)",
                val
            )))
        }
    }

    pub fn is_compressed(&self) -> bool {
        !matches!(self, Self::Identity)
    }
}

/// Resultado tipado da descompressão delimitada com proveniência de tamanhos.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecompressionResult {
    pub encoding: SupportedContentEncoding,
    pub raw_network_bytes: usize,
    pub decompressed_bytes: usize,
    pub decompressed_data: Vec<u8>,
}

/// Descomprime um conjunto de chunks de rede impondo estritamente tetos de bytes comprimidos e descomprimidos.
///
/// INVARIANTES DE SEGURANÇA (ADR-020):
/// 1. Limite na Rede: Chunks comprimidos não podem ultrapassar `max_compressed_bytes` (teto padrão 256 KiB).
/// 2. Limite Descomprimido: O stream descomprimido não pode ultrapassar `max_decompressed_bytes` (teto padrão 1 MiB).
/// 3. Proteção contra Decompression Bomb: A leitura é interrompida imediatamente assim que o limite é excedido,
///    evitando consumo descontrolado de memória RAM (OOM).
/// 4. Falha Fechada: Codificações inválidas, corrompidas ou incompletas abortam com erro explicativo.
pub fn decompress_bounded_stream(
    encoding: SupportedContentEncoding,
    chunks: &[Vec<u8>],
    max_compressed_bytes: usize,
    max_decompressed_bytes: usize,
) -> Result<DecompressionResult, YukiError> {
    let raw_network_bytes: usize = chunks.iter().map(|c| c.len()).sum();

    // 1. Verificação prévia do teto de bytes recebidos pela rede
    if encoding.is_compressed() {
        if raw_network_bytes > max_compressed_bytes {
            return Err(YukiError::ExecutionFailed(format!(
                "Resposta comprimida excedeu o limite máximo de {} bytes (recebidos: {} bytes)",
                max_compressed_bytes, raw_network_bytes
            )));
        }
    } else if raw_network_bytes > max_decompressed_bytes {
        return Err(YukiError::ExecutionFailed(format!(
            "Corpo descomprimido excedeu o limite máximo de {} bytes (recebidos: {} bytes)",
            max_decompressed_bytes, raw_network_bytes
        )));
    }

    // 2. Concatenação segura dos chunks dentro do limite verificado
    let mut compressed_input = Vec::with_capacity(raw_network_bytes);
    for chunk in chunks {
        compressed_input.extend_from_slice(chunk);
    }

    // 3. Descompressão streaming com teto incremental
    let decompressed_data = match encoding {
        SupportedContentEncoding::Identity => compressed_input,
        SupportedContentEncoding::Gzip => {
            let mut decoder = flate2::read::GzDecoder::new(&compressed_input[..]);
            read_stream_with_limit(&mut decoder, max_decompressed_bytes, "gzip")?
        }
        SupportedContentEncoding::Deflate => {
            let mut decoder = flate2::read::DeflateDecoder::new(&compressed_input[..]);
            read_stream_with_limit(&mut decoder, max_decompressed_bytes, "deflate")?
        }
        SupportedContentEncoding::Brotli => {
            let mut decoder = brotli::Decompressor::new(&compressed_input[..], 4096);
            read_stream_with_limit(&mut decoder, max_decompressed_bytes, "brotli")?
        }
    };

    let decompressed_bytes = decompressed_data.len();

    Ok(DecompressionResult {
        encoding,
        raw_network_bytes,
        decompressed_bytes,
        decompressed_data,
    })
}

/// Lê dados de um stream decompositor em blocos de 8 KiB, abortando imediatamente caso exceda o limite.
fn read_stream_with_limit<R: Read>(
    reader: &mut R,
    max_limit: usize,
    encoding_name: &str,
) -> Result<Vec<u8>, YukiError> {
    let mut buffer = Vec::new();
    let mut chunk = [0u8; 8192];
    let mut total_read = 0usize;

    loop {
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                total_read += n;
                if total_read > max_limit {
                    return Err(YukiError::ExecutionFailed(format!(
                        "Corpo descomprimido excedeu o limite máximo de {} bytes ({}: Zip Bomb bloqueado)",
                        max_limit, encoding_name
                    )));
                }
                buffer.extend_from_slice(&chunk[..n]);
            }
            Err(e) => {
                return Err(YukiError::ExecutionFailed(format!(
                    "Falha ao descomprimir stream {}: {}",
                    encoding_name, e
                )));
            }
        }
    }

    Ok(buffer)
}

// ============================================================================
// Funções Auxiliares para Testes e Mocking de Compressão
// ============================================================================

/// Compacta dados utilizando algoritmo Gzip (para testes e fixtures).
pub fn compress_gzip(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// Compacta dados utilizando algoritmo Deflate (para testes e fixtures).
pub fn compress_deflate(data: &[u8]) -> Vec<u8> {
    use std::io::Write;
    let mut encoder =
        flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// Compacta dados utilizando algoritmo Brotli (para testes e fixtures).
pub fn compress_brotli(data: &[u8]) -> Vec<u8> {
    let mut output = Vec::new();
    let params = brotli::enc::BrotliEncoderParams {
        quality: 6,
        ..Default::default()
    };
    brotli::BrotliCompress(&mut &data[..], &mut output, &params).unwrap();
    output
}
