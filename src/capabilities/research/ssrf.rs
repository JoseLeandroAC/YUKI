use crate::contracts::errors::YukiError;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, ToSocketAddrs};
use std::sync::RwLock;

// ============================================================================
// Positive URL Validation & Sanitization
// ============================================================================

/// Valida rigorosamente uma URL para recuperação via `research.fetch` (ADR-020).
///
/// REGRAS DE SEGURANÇA:
/// 1. Comprimento máximo de 2048 caracteres.
/// 2. Esquema exclusivamente HTTP ou HTTPS (preferência HTTPS).
/// 3. Proibição de credenciais embutidas (`username:password@`).
/// 4. Host obrigatório e não vazio.
/// 5. Proibição de nomes de domínio internos/reservados (`localhost`, `.local`, `.internal`, etc.).
/// 6. Proibição de representações ambíguas de IP (octal, hex, dword).
/// 7. Restrição estrita de portas: apenas portas web padrão (80 para HTTP, 443 para HTTPS).
pub fn validate_and_parse_fetch_url(raw_url: &str) -> Result<reqwest::Url, YukiError> {
    let trimmed = raw_url.trim();

    if trimmed.is_empty() {
        return Err(YukiError::InvalidRequest(
            "URL não pode ser vazia".to_string(),
        ));
    }

    if trimmed.len() > 2048 {
        return Err(YukiError::InvalidRequest(format!(
            "URL excede o comprimento máximo permitido de 2048 caracteres (comprimento: {})",
            trimmed.len()
        )));
    }

    let url = reqwest::Url::parse(trimmed)
        .map_err(|e| YukiError::InvalidRequest(format!("Formato de URL inválido: {}", e)))?;

    // 1. Esquema permitido
    let scheme = url.scheme();
    if scheme != "https" && scheme != "http" {
        return Err(YukiError::SecurityViolation(format!(
            "Esquema de URL '{}' não permitido por segurança. Apenas 'https' e 'http' são aceitos.",
            scheme
        )));
    }

    // 2. Proibição de credenciais embutidas
    if !url.username().is_empty() || url.password().is_some() {
        return Err(YukiError::SecurityViolation(
            "Credenciais embutidas na URL (user:pass@) são proibidas por segurança.".to_string(),
        ));
    }

    // Inspeciona se o host na URL bruta continha representação hexadecimal ou ofuscada antes de qualquer normalização do parser
    if let Some(after_scheme) = trimmed.split("://").nth(1) {
        let raw_host = after_scheme
            .split(['/', ':', '?', '#'])
            .next()
            .unwrap_or("")
            .trim();
        if is_ambiguous_ip_encoding(raw_host) {
            return Err(YukiError::SecurityViolation(format!(
                "Representação ambígua ou ofuscada de endereço IP ('{}') é proibida por segurança.",
                raw_host
            )));
        }
    }

    // 3. Validação do host
    let host = url.host_str().ok_or_else(|| {
        YukiError::InvalidRequest("URL deve conter um nome de host válido".to_string())
    })?;

    let host_trimmed = host.trim().to_lowercase();
    if host_trimmed.is_empty() {
        return Err(YukiError::InvalidRequest(
            "Nome de host da URL está vazio".to_string(),
        ));
    }

    // Nomes de domínio especiais / locais reservados
    if is_forbidden_domain_name(&host_trimmed) {
        return Err(YukiError::SecurityViolation(format!(
            "Acesso ao domínio interno ou reservado '{}' é bloqueado por segurança (anti-SSRF).",
            host_trimmed
        )));
    }

    // Proibição de representações numéricas ofuscadas (octal, hex, dword)
    if is_ambiguous_ip_encoding(&host_trimmed) {
        return Err(YukiError::SecurityViolation(format!(
            "Representação ambígua ou ofuscada de endereço IP ('{}') é proibida por segurança.",
            host_trimmed
        )));
    }

    // Se o host for um IP literal (ou foi normalizado para IP pelo parser), valida imediatamente contra a política anti-SSRF
    if let Ok(ip) = host_trimmed.parse::<IpAddr>() {
        is_globally_routable_ip(ip)?;
    }

    // 4. Restrição estrita de portas
    if let Some(port) = url.port() {
        let is_allowed_port = match scheme {
            "http" => port == 80,
            "https" => port == 443,
            _ => false,
        };
        if !is_allowed_port {
            return Err(YukiError::SecurityViolation(format!(
                "Porta não autorizada '{}' para esquema '{}'. Apenas portas padrão (80/443) são permitidas.",
                port, scheme
            )));
        }
    }

    Ok(url)
}

/// Identifica sufixos e nomes de domínio reservados ou internos.
fn is_forbidden_domain_name(host: &str) -> bool {
    let lower = host.to_lowercase();
    if lower == "localhost" {
        return true;
    }

    let forbidden_suffixes = [
        ".localhost",
        ".local",
        ".internal",
        ".lan",
        ".home.arpa",
        ".test",
        ".example",
        ".invalid",
        ".corp",
        ".onion",
    ];

    for suffix in &forbidden_suffixes {
        if lower.ends_with(suffix) {
            return true;
        }
    }

    false
}

/// Detecta formatos de IP ofuscados como números inteiros isolados (dword),
/// notações com zeros à esquerda (octal), hexadecimais em partes ou IPs numéricos incompletos.
fn is_ambiguous_ip_encoding(host: &str) -> bool {
    let trimmed = host.trim();
    if trimmed.is_empty() {
        return false;
    }

    // Hexadecimal geral
    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
        return true;
    }

    // Dword (número puro como 2130706433)
    if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.contains('.') {
        return true;
    }

    // Componentes de IPv4 pontuados
    if trimmed.contains('.') {
        let parts: Vec<&str> = trimmed.split('.').collect();
        // Verifica se todos os segmentos são numéricos (dígitos ou hex com 0x)
        let all_numeric_parts = parts.iter().all(|p| {
            p.chars().all(|c| c.is_ascii_digit())
                || ((p.starts_with("0x") || p.starts_with("0X"))
                    && p.len() > 2
                    && p[2..].chars().all(|c| c.is_ascii_hexdigit()))
        });

        if all_numeric_parts {
            // Se tem menos de 4 partes numéricas (ex: 127.1, 10.1, 127.0.1)
            if parts.len() < 4 {
                return true;
            }
            // Se tem partes com zero à esquerda (octal em C/POSIX) ou hex em parte (ex: 127.0.0.0x1)
            for p in &parts {
                if (p.len() > 1
                    && p.starts_with('0')
                    && !p.starts_with("0x")
                    && !p.starts_with("0X"))
                    || p.starts_with("0x")
                    || p.starts_with("0X")
                {
                    return true;
                }
            }
        }
    }

    false
}

// ============================================================================
// Exhaustive Positive IP Policy (Anti-SSRF IPv4, IPv6 & IPv4-Mapped)
// ============================================================================

/// Valida se um endereço IP é globalmente roteável e público (ADR-020).
///
/// Rejeita exaustivamente:
/// - Loopback (IPv4 127/8, IPv6 ::1)
/// - Privado (RFC 1918: 10/8, 172.16/12, 192.168/16; IPv6 ULA fc00::/7)
/// - Link-Local (IPv4 169.254/16; IPv6 fe80::/10)
/// - Cloud Metadata (`169.254.169.254`, `[fd00:ec2::254]`, etc.)
/// - CGNAT / Shared (RFC 6598: 100.64/10)
/// - Rede atual (0.0.0.0/8)
/// - Broadcast (255.255.255.255)
/// - Multicast (IPv4 224/4; IPv6 ff00::/8)
/// - Reservado para uso futuro (240.0.0.0/4)
/// - Redes de documentação e teste (192.0.2/24, 198.51.100/24, 203.0.113/24, 2001:db8::/32)
/// - Benchmarking (198.18/15)
/// - 6to4 Relay (192.88.99/24, 2002::/16)
/// - IETF assignments (192.0.0/24)
/// - IPv4-mapped IPv6 (`::ffff:0:0/96`) mapeado para qualquer IP privado
pub fn is_globally_routable_ip(ip: IpAddr) -> Result<(), YukiError> {
    match ip {
        IpAddr::V4(ipv4) => validate_ipv4(ipv4),
        IpAddr::V6(ipv6) => validate_ipv6(ipv6),
    }
}

fn validate_ipv4(ipv4: Ipv4Addr) -> Result<(), YukiError> {
    let [a, b, c, _d] = ipv4.octets();

    // 0.0.0.0/8 (Rede atual RFC 1122)
    if a == 0 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 na rede local (0.0.0.0/8): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 10.0.0.0/8 (Privada RFC 1918)
    if a == 10 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 privado (10.0.0.0/8): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 100.64.0.0/10 (Carrier-Grade NAT RFC 6598)
    if a == 100 && (64..=127).contains(&b) {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 CGNAT (100.64.0.0/10): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 127.0.0.0/8 (Loopback)
    if a == 127 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 loopback (127.0.0.0/8): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 169.254.0.0/16 (Link-Local e Cloud Metadata: 169.254.169.254)
    if a == 169 && b == 254 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 link-local/cloud-metadata (169.254.0.0/16): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 172.16.0.0/12 (Privada RFC 1918)
    if a == 172 && (16..=31).contains(&b) {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 privado (172.16.0.0/12): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 192.0.0.0/24 (IETF Protocol Assignments RFC 6890)
    if a == 192 && b == 0 && c == 0 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 reservado IETF (192.0.0.0/24): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 192.0.2.0/24 (TEST-NET-1 RFC 5737)
    if a == 192 && b == 0 && c == 2 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 documentação TEST-NET-1 (192.0.2.0/24): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 192.88.99.0/24 (6to4 Relay Anycast RFC 7526)
    if a == 192 && b == 88 && c == 99 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 6to4 relay (192.88.99.0/24): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 192.168.0.0/16 (Privada RFC 1918)
    if a == 192 && b == 168 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 privado (192.168.0.0/16): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 198.18.0.0/15 (Benchmarking RFC 2544)
    if a == 198 && (b == 18 || b == 19) {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 benchmarking (198.18.0.0/15): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 198.51.100.0/24 (TEST-NET-2 RFC 5737)
    if a == 198 && b == 51 && c == 100 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 documentação TEST-NET-2 (198.51.100.0/24): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 203.0.113.0/24 (TEST-NET-3 RFC 5737)
    if a == 203 && b == 0 && c == 113 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 documentação TEST-NET-3 (203.0.113.0/24): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 224.0.0.0/4 (Multicast RFC 5771)
    if (224..=239).contains(&a) {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 multicast (224.0.0.0/4): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    // 240.0.0.0/4 (Reservado / Broadcast RFC 1112)
    if a >= 240 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv4 reservado/broadcast (240.0.0.0/4): '{}' bloqueado por SSRF.",
            ipv4
        )));
    }

    Ok(())
}

fn validate_ipv6(ipv6: Ipv6Addr) -> Result<(), YukiError> {
    let segments = ipv6.segments();

    // 1. IPv4-Mapped IPv6 (::ffff:0:0/96)
    // CRÍTICO: Desencapsular e validar estritamente contra as regras de IPv4!
    if segments[0] == 0
        && segments[1] == 0
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0xffff
    {
        let ipv4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return validate_ipv4(ipv4).map_err(|e| {
            YukiError::SecurityViolation(format!(
                "IPv4-Mapped IPv6 ('{}') encapsula IPv4 bloqueado: {}",
                ipv6, e
            ))
        });
    }

    // IPv4-Compatible IPv6 (::0:0/96 depreciado)
    if segments[0] == 0
        && segments[1] == 0
        && segments[2] == 0
        && segments[3] == 0
        && segments[4] == 0
        && segments[5] == 0
        && (segments[6] != 0 || segments[7] != 0 && segments[7] != 1)
    {
        let ipv4 = Ipv4Addr::new(
            (segments[6] >> 8) as u8,
            (segments[6] & 0xff) as u8,
            (segments[7] >> 8) as u8,
            (segments[7] & 0xff) as u8,
        );
        return validate_ipv4(ipv4).map_err(|e| {
            YukiError::SecurityViolation(format!(
                "IPv4-Compatible IPv6 ('{}') encapsula IPv4 bloqueado: {}",
                ipv6, e
            ))
        });
    }

    // 2. IPv6 Loopback (::1/128)
    if ipv6.is_loopback() {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 loopback (::1): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    // 3. IPv6 Unspecified (::/128)
    if ipv6.is_unspecified() {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 não especificado (::): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    // 4. Discard Prefix (100::/64 RFC 6666)
    if segments[0] == 0x0100 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 discard prefix (100::/64): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    // 5. Documentação (2001:db8::/32 RFC 3849)
    if segments[0] == 0x2001 && segments[1] == 0x0db8 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 de documentação (2001:db8::/32): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    // 6. 6to4 Prefix (2002::/16 RFC 7526)
    if segments[0] == 0x2002 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 6to4 (2002::/16): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    // 7. Unique Local Addresses (fc00::/7 RFC 4193)
    if (segments[0] & 0xfe00) == 0xfc00 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 ULA privado (fc00::/7): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    // 8. Link-Local Unicast (fe80::/10 RFC 4291)
    if (segments[0] & 0xffc0) == 0xfe80 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 link-local (fe80::/10): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    // 8.5. Site-Local Depreciado (fec0::/10 RFC 3879)
    if (segments[0] & 0xffc0) == 0xfec0 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 site-local depreciado (fec0::/10): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    // 9. Multicast (ff00::/8 RFC 4291)
    if (segments[0] & 0xff00) == 0xff00 {
        return Err(YukiError::SecurityViolation(format!(
            "Endereço IPv6 multicast (ff00::/8): '{}' bloqueado por SSRF.",
            ipv6
        )));
    }

    Ok(())
}

// ============================================================================
// DNS Resolution & Socket Pinning (DNS Rebinding Defense)
// ============================================================================

/// Trait abstrato para resolução de nomes DNS (permite mock seguro em testes).
pub trait DnsResolver: Send + Sync {
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, YukiError>;
}

/// Resolução DNS real do sistema operacional.
pub struct SystemDnsResolver;

impl DnsResolver for SystemDnsResolver {
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, YukiError> {
        let socket_str = format!("{}:80", host);
        let addrs = socket_str.to_socket_addrs().map_err(|e| {
            YukiError::ExecutionFailed(format!(
                "Falha na resolução DNS para o host '{}': {}",
                host, e
            ))
        })?;

        let ips: Vec<IpAddr> = addrs.map(|s| s.ip()).collect();
        if ips.is_empty() {
            return Err(YukiError::ExecutionFailed(format!(
                "Nenhum endereço IP retornado na resolução DNS para '{}'",
                host
            )));
        }

        Ok(ips)
    }
}

/// Resolução DNS simulada em memória para testes offline determinísticos.
pub struct MockDnsResolver {
    records: RwLock<HashMap<String, Vec<IpAddr>>>,
}

impl MockDnsResolver {
    pub fn new() -> Self {
        Self {
            records: RwLock::new(HashMap::new()),
        }
    }

    pub fn set_host_ips(&self, host: impl Into<String>, ips: Vec<IpAddr>) {
        self.records
            .write()
            .unwrap()
            .insert(host.into().to_lowercase(), ips);
    }
}

impl Default for MockDnsResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl DnsResolver for MockDnsResolver {
    fn resolve(&self, host: &str) -> Result<Vec<IpAddr>, YukiError> {
        let key = host.to_lowercase();
        if let Some(ips) = self.records.read().unwrap().get(&key) {
            if ips.is_empty() {
                return Err(YukiError::ExecutionFailed(format!(
                    "DNS mock retornou lista vazia de IPs para '{}'",
                    host
                )));
            }
            return Ok(ips.clone());
        }

        Err(YukiError::ExecutionFailed(format!(
            "Host '{}' não encontrado no resolvedor DNS simulado",
            host
        )))
    }
}

/// Resolve e amarra (*pins*) o host ao endereço IP público validado.
///
/// INVARIANTE DE SEGURANÇA (Prevenção de DNS Rebinding):
/// 1. Se o host for um IP literal: valida o IP diretamente.
/// 2. Se for um domínio: resolve o DNS e inspeciona **TODOS** os IPs retornados.
/// 3. Se **QUALQUER** IP pertencer a uma faixa privada/proibida, a conexão é REJEITADA integralmente (fail-closed).
/// 4. Retorna o `SocketAddr` com o IP aprovado amarrado à porta de destino.
pub fn resolve_and_pin_host(
    host: &str,
    port: u16,
    resolver: &dyn DnsResolver,
) -> Result<SocketAddr, YukiError> {
    // 1. Host é um literal de IP?
    if let Ok(ip) = host.parse::<IpAddr>() {
        is_globally_routable_ip(ip)?;
        return Ok(SocketAddr::new(ip, port));
    }

    // 2. Resolução DNS com checagem exaustiva de TODOS os IPs retornados
    let resolved_ips = resolver.resolve(host)?;
    if resolved_ips.is_empty() {
        return Err(YukiError::ExecutionFailed(format!(
            "Resolução DNS não retornou endereços para '{}'",
            host
        )));
    }

    // Regra de falha fechada: Se QUALQUER endereço for proibido, recusa a requisição inteira!
    for ip in &resolved_ips {
        if let Err(e) = is_globally_routable_ip(*ip) {
            return Err(YukiError::SecurityViolation(format!(
                "Host '{}' resolveu para endereço IP bloqueado por política SSRF ({}): {}",
                host, ip, e
            )));
        }
    }

    // Pinning: Seleciona o primeiro IP validado
    let approved_ip = resolved_ips[0];
    Ok(SocketAddr::new(approved_ip, port))
}
