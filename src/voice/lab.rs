use crate::contracts::identifiers::generate_unique_id;
use crate::contracts::voice::{AudioOutput, TextToSpeechProvider, VoiceError};
use crate::voice::tts::{
    get_piper_voice_catalog, LocalAcousticTtsProvider, LocalPiperConfig, LocalPiperTtsProvider,
};
use std::collections::HashMap;
use std::path::PathBuf;

/// Frase padronizada de teste do Laboratório Yuki Voice.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StandardizedPhrase {
    pub id: &'static str,
    pub category: &'static str,
    pub text: &'static str,
    pub description: &'static str,
}

/// Conjunto fixo canônico de frases para avaliação comparativa e calibração vocal da Yuki.
pub const STANDARDIZED_PHRASES: &[StandardizedPhrase] = &[
    StandardizedPhrase {
        id: "saudacao",
        category: "Saudação e Identidade",
        text: "Olá! Eu sou a Yuki, sua assistente pessoal no Windows. O que faremos hoje?",
        description: "Avalia calor, acolhimento inicial e cadência de introdução da assistente.",
    },
    StandardizedPhrase {
        id: "explicacao_tecnica",
        category: "Explicação Técnica",
        text: "O subsistema de voz agora utiliza WASAPI para captura e reprodução de áudio de alta precisão.",
        description: "Avalia articulação de terminologia técnica e clareza fonêmica.",
    },
    StandardizedPhrase {
        id: "conversa_natural",
        category: "Conversação Cotidiana",
        text: "Estou pronta para ajudar você com tarefas, pesquisas e automações no sistema.",
        description: "Avalia naturalidade melódica, modulação e fluidez cotidiana.",
    },
    StandardizedPhrase {
        id: "aviso",
        category: "Alerta e Governança",
        text: "Atenção: uma ação de alto impacto requer sua confirmação explícita antes de continuar.",
        description: "Avalia autoridade serena, firmeza e seriedade sem agressividade.",
    },
    StandardizedPhrase {
        id: "pronuncia",
        category: "Desafio Fonético",
        text: "Inicializando o subsistema de áudio na arquitetura x86_64 com latência mínima.",
        description: "Avalia pronúncia complexa de acrônimos, números e encontros consonantais.",
    },
];

/// Retorna a lista de frases de teste padronizadas.
pub fn get_standardized_phrases() -> &'static [StandardizedPhrase] {
    STANDARDIZED_PHRASES
}

/// Localiza uma frase padronizada pelo seu identificador (ex: "saudacao").
pub fn find_phrase(id: &str) -> Option<&'static StandardizedPhrase> {
    STANDARDIZED_PHRASES
        .iter()
        .find(|p| p.id.eq_ignore_ascii_case(id))
}

/// Metadados persistidos de uma amostra gerada no Laboratório.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceSampleMetadata {
    pub sample_id: String,
    pub voice_id: String,
    pub phrase_id: String,
    pub phrase_text: String,
    pub length_scale: f32,
    pub noise_scale: f32,
    pub noise_w: f32,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub channels: u16,
    pub file_path: String,
    pub is_neural: bool,
    pub timestamp: String,
}

/// Registro estruturado de avaliação auditada de uma voz por operador humano.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceEvaluation {
    pub evaluation_id: String,
    pub voice_id: String,
    pub phrase_id: Option<String>,
    pub naturalness: u8,
    pub tonality: u8,
    pub smoothness: u8,
    pub expressiveness: u8,
    pub rhythm: u8,
    pub average_score: f32,
    pub approved: bool,
    pub operator_notes: String,
    pub evaluated_at: String,
}

/// Especificação técnica do Perfil Vocal da Yuki v0.1.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct YukiVocalProfile {
    pub version: String,
    pub target_identity: String,
    pub target_f0_hz: String,
    pub target_cadence: String,
    pub recommended_length_scale: f32,
    pub recommended_noise_scale: f32,
    pub recommended_noise_w: f32,
    pub min_target_naturalness: u8,
    pub min_target_tonality: u8,
    pub min_target_smoothness: u8,
    pub min_target_expressiveness: u8,
    pub min_target_rhythm: u8,
}

impl Default for YukiVocalProfile {
    fn default() -> Self {
        Self {
            version: "v0.1".to_string(),
            target_identity: "Feminina, jovem-adulta (aprox. 20-25 anos), calorosa, serena, profissional mas amigável".to_string(),
            target_f0_hz: "~200-240 Hz (faixa média-alta feminina)".to_string(),
            target_cadence: "Levemente pausada, clara, sem pressa (ritmo ~ 0.9x velocidade normal)".to_string(),
            recommended_length_scale: 1.14,
            recommended_noise_scale: 0.55,
            recommended_noise_w: 0.70,
            min_target_naturalness: 80,
            min_target_tonality: 75,
            min_target_smoothness: 80,
            min_target_expressiveness: 70,
            min_target_rhythm: 75,
        }
    }
}

/// Resumo estatístico das avaliações de uma voz no histórico.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VoiceSummaryStats {
    pub voice_id: String,
    pub total_evaluations: usize,
    pub avg_naturalness: f32,
    pub avg_tonality: f32,
    pub avg_smoothness: f32,
    pub avg_expressiveness: f32,
    pub avg_rhythm: f32,
    pub avg_overall: f32,
    pub approvals_count: usize,
}

/// Retorna o diretório de destino de amostras de voz.
pub fn samples_dir() -> PathBuf {
    if let Ok(path) = std::env::var("YUKI_VOICE_SAMPLES_DIR") {
        PathBuf::from(path)
    } else if let Ok(data_dir) = std::env::var("YUKI_DATA_DIR") {
        PathBuf::from(data_dir).join("voice_samples")
    } else {
        PathBuf::from("data/voice_samples")
    }
}

/// Retorna o caminho do arquivo de avaliações persistidas.
pub fn evaluations_file() -> PathBuf {
    if let Ok(path) = std::env::var("YUKI_VOICE_EVALS_FILE") {
        PathBuf::from(path)
    } else if let Ok(data_dir) = std::env::var("YUKI_DATA_DIR") {
        PathBuf::from(data_dir).join("voice_evaluations.json")
    } else {
        PathBuf::from("data/voice_evaluations.json")
    }
}

/// Gera uma amostra de áudio para a voz e frase solicitadas, salvando WAV e metadados no diretório de amostras.
pub async fn generate_voice_sample(
    voice_id: &str,
    phrase_id: &str,
    custom_length_scale: Option<f32>,
    custom_noise_scale: Option<f32>,
    custom_noise_w: Option<f32>,
) -> Result<(AudioOutput, VoiceSampleMetadata), VoiceError> {
    let phrase_obj = find_phrase(phrase_id);
    let phrase_text = phrase_obj
        .map(|p| p.text.to_string())
        .unwrap_or_else(|| phrase_id.to_string());
    let canonical_phrase_id = phrase_obj
        .map(|p| p.id.to_string())
        .unwrap_or_else(|| "custom".to_string());

    let profile = YukiVocalProfile::default();
    let length_scale = custom_length_scale.unwrap_or(profile.recommended_length_scale);
    let noise_scale = custom_noise_scale.unwrap_or(profile.recommended_noise_scale);
    let noise_w = custom_noise_w.unwrap_or(profile.recommended_noise_w);

    let (audio_out, is_neural) = if voice_id.eq_ignore_ascii_case("acoustic") {
        let acoustic = LocalAcousticTtsProvider::default();
        let out = acoustic.synthesize(&phrase_text, None).await?;
        (out, false)
    } else {
        let catalog = get_piper_voice_catalog();
        let model_info = catalog
            .into_iter()
            .find(|m| m.id.eq_ignore_ascii_case(voice_id))
            .ok_or_else(|| {
                VoiceError::ProviderUnavailable(format!(
                    "Voz '{}' não encontrada no catálogo. Utilize 'yuki voice voices' para verificar opções. {}",
                    voice_id,
                    LocalPiperTtsProvider::download_instructions()
                ))
            })?;

        if !model_info.is_installed {
            return Err(VoiceError::ProviderUnavailable(format!(
                "Modelo '{}' não está baixado localmente. {}\nInstruções de download:\nURL ONNX: {}\nURL JSON: {}",
                voice_id,
                LocalPiperTtsProvider::download_instructions(),
                model_info.download_onnx_url,
                model_info.download_json_url
            )));
        }

        let model_path = model_info.local_path.ok_or_else(|| {
            VoiceError::ProviderUnavailable(format!(
                "Caminho do modelo local '{}' não encontrado.",
                voice_id
            ))
        })?;

        // Auto-detect config json correspondente
        let json_candidate_1 = PathBuf::from(format!("{}.json", model_path.display()));
        let json_candidate_2 = model_path.with_extension("json");
        let config_json = if json_candidate_1.is_file() {
            Some(json_candidate_1)
        } else if json_candidate_2.is_file() {
            Some(json_candidate_2)
        } else {
            None
        };

        let config = LocalPiperConfig {
            model_path: Some(model_path.clone()),
            config_json,
            length_scale,
            noise_scale,
            noise_w,
            ..Default::default()
        };

        let piper = LocalPiperTtsProvider::new(config);
        let out = piper.synthesize(&phrase_text, None).await?;
        (out, true)
    };

    let dir = samples_dir();
    std::fs::create_dir_all(&dir)
        .map_err(|e| VoiceError::Io(format!("Falha ao criar diretório de amostras: {}", e)))?;

    let now_utc = crate::contracts::identifiers::now_utc();
    let ts_str = now_utc.format("%Y%m%d_%H%M%S").to_string();
    let filename = format!("{}_{}_{}.wav", voice_id, canonical_phrase_id, ts_str);
    let sample_path = dir.join(&filename);

    std::fs::write(&sample_path, &audio_out.payload)
        .map_err(|e| VoiceError::Io(format!("Falha ao salvar amostra de áudio na pasta: {}", e)))?;

    let metadata = VoiceSampleMetadata {
        sample_id: generate_unique_id("sample"),
        voice_id: voice_id.to_string(),
        phrase_id: canonical_phrase_id,
        phrase_text,
        length_scale,
        noise_scale,
        noise_w,
        duration_ms: audio_out.duration_ms,
        sample_rate: audio_out.format.sample_rate,
        channels: audio_out.format.channels,
        file_path: sample_path.to_string_lossy().to_string(),
        is_neural,
        timestamp: now_utc.to_rfc3339(),
    };

    let meta_file = dir.join(format!(
        "{}_{}_{}.meta.json",
        voice_id, metadata.phrase_id, ts_str
    ));
    if let Ok(meta_json) = serde_json::to_string_pretty(&metadata) {
        let _ = std::fs::write(meta_file, meta_json);
    }

    Ok((audio_out, metadata))
}

/// Carrega a lista de avaliações persistidas do arquivo JSON.
pub fn load_evaluations() -> Result<Vec<VoiceEvaluation>, VoiceError> {
    let file = evaluations_file();
    if !file.exists() {
        return Ok(Vec::new());
    }

    let content = std::fs::read_to_string(&file)
        .map_err(|e| VoiceError::Io(format!("Falha ao ler arquivo de avaliações: {}", e)))?;

    if content.trim().is_empty() {
        return Ok(Vec::new());
    }

    let list: Vec<VoiceEvaluation> = serde_json::from_str(&content)
        .map_err(|e| VoiceError::Io(format!("Falha ao deserializar avaliações JSON: {}", e)))?;

    Ok(list)
}

/// Salva uma nova avaliação de forma atômica e segura no arquivo JSON de histórico.
pub fn save_evaluation(eval: VoiceEvaluation) -> Result<(), VoiceError> {
    let file = evaluations_file();
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            VoiceError::Io(format!("Falha ao criar diretório para avaliações: {}", e))
        })?;
    }

    let mut list = load_evaluations()?;
    list.push(eval);

    let json_bytes = serde_json::to_vec_pretty(&list)
        .map_err(|e| VoiceError::Io(format!("Falha ao serializar avaliações: {}", e)))?;

    let temp_file = file.with_extension(format!("tmp.{}", generate_unique_id("eval")));
    std::fs::write(&temp_file, &json_bytes)
        .map_err(|e| VoiceError::Io(format!("Falha ao escrever avaliação temporária: {}", e)))?;

    std::fs::rename(&temp_file, &file).map_err(|e| {
        let _ = std::fs::remove_file(&temp_file);
        VoiceError::Io(format!("Falha atômica ao salvar avaliações: {}", e))
    })?;

    Ok(())
}

/// Computa o resumo estatístico consolidado das avaliações por modelo de voz.
pub fn compute_evaluations_summary() -> HashMap<String, VoiceSummaryStats> {
    let list = load_evaluations().unwrap_or_default();
    let mut grouped: HashMap<String, Vec<VoiceEvaluation>> = HashMap::new();

    for ev in list {
        grouped.entry(ev.voice_id.clone()).or_default().push(ev);
    }

    let mut summary = HashMap::new();
    for (voice_id, evals) in grouped {
        let count = evals.len();
        if count == 0 {
            continue;
        }

        let sum_nat: u32 = evals.iter().map(|e| e.naturalness as u32).sum();
        let sum_ton: u32 = evals.iter().map(|e| e.tonality as u32).sum();
        let sum_smo: u32 = evals.iter().map(|e| e.smoothness as u32).sum();
        let sum_exp: u32 = evals.iter().map(|e| e.expressiveness as u32).sum();
        let sum_rhy: u32 = evals.iter().map(|e| e.rhythm as u32).sum();
        let sum_ovr: f32 = evals.iter().map(|e| e.average_score).sum();
        let approvals = evals.iter().filter(|e| e.approved).count();

        let stats = VoiceSummaryStats {
            voice_id: voice_id.clone(),
            total_evaluations: count,
            avg_naturalness: sum_nat as f32 / count as f32,
            avg_tonality: sum_ton as f32 / count as f32,
            avg_smoothness: sum_smo as f32 / count as f32,
            avg_expressiveness: sum_exp as f32 / count as f32,
            avg_rhythm: sum_rhy as f32 / count as f32,
            avg_overall: sum_ovr / count as f32,
            approvals_count: approvals,
        };
        summary.insert(voice_id, stats);
    }

    summary
}
