use crate::contracts::memory::{
    MemoryCandidate, MemoryError, MemoryId, MemoryPolicy, MemoryRecord, MemoryRepository,
};
use std::sync::Arc;

/// Controlador governado de admissão e ciclo de vida de memórias.
///
/// INVARIANTE INEGOCIÁVEL:
/// Model Output != Memory != Persistence.
/// Nenhuma inferência do modelo é persistida diretamente sem avaliação de política e sanitização.
pub struct MemoryController {
    repository: Arc<dyn MemoryRepository>,
    policy: MemoryPolicy,
}

impl MemoryController {
    pub fn new(repository: Arc<dyn MemoryRepository>, policy: MemoryPolicy) -> Self {
        Self { repository, policy }
    }

    pub fn policy(&self) -> &MemoryPolicy {
        &self.policy
    }

    /// Avalia um candidato a memória contra as políticas da plataforma e, se aprovado,
    /// persiste-o no repositório durável.
    pub fn evaluate_and_commit(
        &self,
        candidate: &MemoryCandidate,
    ) -> Result<MemoryRecord, MemoryError> {
        // 1. Avaliação contra as regras de retenção e privacidade da política
        self.policy.evaluate_candidate(candidate)?;

        // 2. Conversão para registro tipado com sanitização preventiva
        let record = candidate.to_record()?;

        // 3. Persistência durável
        self.repository.store(&record)?;

        Ok(record)
    }

    /// Revoga uma memória por solicitação do usuário.
    pub fn revoke_memory(&self, id: &MemoryId) -> Result<(), MemoryError> {
        self.repository.revoke(id)
    }

    /// Exclusão física permanente sob direito ao esquecimento.
    pub fn forget_permanently(&self, id: &MemoryId) -> Result<(), MemoryError> {
        self.repository.delete_permanently(id)
    }
}
