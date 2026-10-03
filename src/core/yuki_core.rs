use crate::audit::event_store::{EventStore, InMemoryEventStore};
use crate::capabilities::registry::CapabilityRegistry;
use crate::context::builder::ContextBuilder;
use crate::contracts::authorization::{AuthorizationDecision, AuthorizationRequest};
use crate::contracts::capability::RiskClass;
use crate::contracts::context::ContextRequest;
use crate::contracts::errors::YukiError;
use crate::contracts::events::{AuditEvent, EventType};
use crate::contracts::execution::{ExecutionRequest, OperationState};
use crate::contracts::identifiers::{
    now_utc, AttemptId, CapabilityId, CausationId, CorrelationId, EvidenceId, OperationId,
};
use crate::contracts::input::UserInput;
use crate::contracts::output::YukiResult;
use crate::contracts::verification::Evidence;
use crate::execution::executor::Executor;
use crate::models::config::ModelGatewayConfig;
use crate::models::mock::MockModelProvider;
use crate::models::proposal_parser::ProposalParser;
use crate::models::provider::{HealthStatus, ModelProvider, ModelRequest};
use crate::security::authorization::SecurityController;
use crate::verification::verifier::Verifier;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SystemHealth {
    pub core_ok: bool,
    pub context_ok: bool,
    pub model_ok: bool,
    pub registry_ok: bool,
    pub security_ok: bool,
    pub execution_ok: bool,
    pub verification_ok: bool,
    pub audit_ok: bool,
}

impl SystemHealth {
    pub fn is_all_ok(&self) -> bool {
        self.core_ok
            && self.context_ok
            && self.model_ok
            && self.registry_ok
            && self.security_ok
            && self.execution_ok
            && self.verification_ok
            && self.audit_ok
    }
}

pub struct YukiCore {
    pub model_provider: Arc<dyn ModelProvider>,
    pub capability_registry: Arc<CapabilityRegistry>,
    pub security_controller: Arc<SecurityController>,
    pub executor: Arc<Executor>,
    pub verifier: Arc<Verifier>,
    pub event_store: Arc<dyn EventStore>,
}

impl YukiCore {
    pub fn new() -> Self {
        Self {
            model_provider: Arc::new(MockModelProvider::new()),
            capability_registry: Arc::new(CapabilityRegistry::new()),
            security_controller: Arc::new(SecurityController::new()),
            executor: Arc::new(Executor::new()),
            verifier: Arc::new(Verifier::new()),
            event_store: Arc::new(InMemoryEventStore::new()),
        }
    }

    pub fn with_components(
        model_provider: Arc<dyn ModelProvider>,
        capability_registry: Arc<CapabilityRegistry>,
        security_controller: Arc<SecurityController>,
        executor: Arc<Executor>,
        verifier: Arc<Verifier>,
        event_store: Arc<dyn EventStore>,
    ) -> Self {
        Self {
            model_provider,
            capability_registry,
            security_controller,
            executor,
            verifier,
            event_store,
        }
    }

    /// Health diagnostic of all fundamental components
    pub fn health(&self) -> SystemHealth {
        let model_ok = self.model_provider.health() == HealthStatus::Healthy;
        let registry_ok = self
            .capability_registry
            .has_capability(&CapabilityId::new("system.echo"));
        let audit_ok = self
            .event_store
            .record(AuditEvent::new(
                EventType::InputReceived,
                CorrelationId::new(),
                CausationId::new("health_check"),
                serde_json::json!({"check": "ping"}),
                "health_system",
            ))
            .is_ok();

        SystemHealth {
            core_ok: true,
            context_ok: true,
            model_ok,
            registry_ok,
            security_ok: true,
            execution_ok: true,
            verification_ok: true,
            audit_ok,
        }
    }

    /// Asynchronous processing pipeline (Marco 2 Core Orchestration).
    ///
    /// Preserves Constitutional Invariants:
    /// - Capability != Permission != Authorization != Execution
    /// - Model Output != Command != Authorization
    /// - Think != Authorize != Execute
    /// - Data != Instruction
    /// - External model output is UNTRUSTED DATA
    pub async fn process_input_async(&self, input: UserInput) -> Result<YukiResult, YukiError> {
        let request_id = input.request_id.clone();
        let correlation_id = CorrelationId::from_request(&request_id);

        // 1. Audit Input Received
        self.event_store.record(AuditEvent::new(
            EventType::InputReceived,
            correlation_id.clone(),
            CausationId::new(request_id.to_string()),
            serde_json::json!({
                "content_length": input.content.len(),
                "timestamp": input.timestamp
            }),
            "yuki_core",
        ))?;

        // 2. Build Context Object (Deterministic and synchronous)
        let context_req = ContextRequest {
            purpose: "user_turn".to_string(),
            user_input: input.content.clone(),
            source: "user".to_string(),
        };
        let context = ContextBuilder::build(&context_req)?;

        self.event_store.record(AuditEvent::new(
            EventType::ContextBuilt,
            correlation_id.clone(),
            CausationId::new(request_id.to_string()),
            serde_json::json!({
                "context_id": context.context_id.0,
                "purpose": context.purpose
            }),
            "context_builder",
        ))?;

        // 3. Model Provider (Cognitive Layer - Asynchronous Boundary)
        let model_req = ModelRequest::new(
            input.content.clone(),
            context.context_id.clone(),
            context.purpose.clone(),
        );

        let model_resp = self.model_provider.generate(&model_req).await?;

        let has_proposal =
            model_resp.capability_proposal.is_some() || model_resp.candidate_proposal.is_some();
        self.event_store.record(AuditEvent::new(
            EventType::ModelInvoked,
            correlation_id.clone(),
            CausationId::new(context.context_id.0.clone()),
            serde_json::json!({
                "provider": self.model_provider.metadata().provider_name,
                "has_proposal": has_proposal
            }),
            "yuki_core",
        ))?;

        // 4. Capability Proposal Handling & ProposalParser validation
        let proposal = if let Some(p) = model_resp.capability_proposal {
            p
        } else if let Some(candidate) = model_resp.candidate_proposal {
            // Validate untrusted model candidate through ProposalParser
            ProposalParser::parse(
                candidate,
                &model_req,
                &self.capability_registry,
                &ModelGatewayConfig::default(),
                model_resp.provider_response_id.clone(),
            )?
        } else {
            // If model didn't propose any capability, return raw conversational output
            let res = YukiResult::success(
                request_id.clone(),
                correlation_id.clone(),
                None,
                model_resp.raw_content,
                None,
            );
            self.event_store.record(AuditEvent::new(
                EventType::ResponseProduced,
                correlation_id,
                CausationId::new(request_id.to_string()),
                serde_json::json!({ "status": "DirectText" }),
                "yuki_core",
            ))?;
            return Ok(res);
        };

        let operation_id = OperationId::new();
        self.event_store.record(AuditEvent::new(
            EventType::CapabilityProposed,
            correlation_id.clone(),
            CausationId::new(request_id.to_string()),
            serde_json::json!({
                "operation_id": operation_id.0,
                "capability_id": proposal.capability_id.0,
                "reasoning": proposal.reasoning
            }),
            "model_provider",
        ))?;

        // 5. Security Controller Authorization
        // Invariant: Model output != Authorization! Explicit policy evaluation is required.
        let risk_class = self
            .capability_registry
            .get_manifest(&proposal.capability_id)
            .map(|m| m.risk_class)
            .unwrap_or(RiskClass::High);

        let auth_req = AuthorizationRequest {
            operation_id: operation_id.clone(),
            capability_id: proposal.capability_id.clone(),
            context_id: context.context_id.clone(),
            caller_id: "yuki_core".to_string(),
            input_summary: proposal.parameters.clone(),
            risk_class: risk_class.clone(),
        };

        self.event_store.record(AuditEvent::new(
            EventType::AuthorizationRequested,
            correlation_id.clone(),
            CausationId::new(operation_id.0.clone()),
            serde_json::json!({
                "operation_id": operation_id.0,
                "capability_id": proposal.capability_id.0
            }),
            "security_controller",
        ))?;

        let auth_decision = self
            .security_controller
            .authorize(&auth_req, &self.capability_registry)?;

        let auth_token = match auth_decision {
            AuthorizationDecision::Allow { token, .. } => {
                self.event_store.record(AuditEvent::new(
                    EventType::AuthorizationGranted,
                    correlation_id.clone(),
                    CausationId::new(operation_id.0.clone()),
                    serde_json::json!({
                        "decision": "Allow",
                        "risk_class": format!("{:?}", risk_class)
                    }),
                    "security_controller",
                ))?;
                token
            }
            AuthorizationDecision::RequiresApproval { reason } => {
                self.event_store.record(AuditEvent::new(
                    EventType::AuthorizationDenied,
                    correlation_id.clone(),
                    CausationId::new(operation_id.0.clone()),
                    serde_json::json!({
                        "operation_id": operation_id.0,
                        "requires_approval": true,
                        "reason": reason
                    }),
                    "security_controller",
                ))?;
                return Ok(YukiResult::denied(
                    request_id,
                    correlation_id,
                    format!("Ação requer aprovação humana: {}", reason),
                ));
            }
            AuthorizationDecision::Deny { reason } => {
                self.event_store.record(AuditEvent::new(
                    EventType::AuthorizationDenied,
                    correlation_id.clone(),
                    CausationId::new(operation_id.0.clone()),
                    serde_json::json!({
                        "operation_id": operation_id.0,
                        "reason": reason
                    }),
                    "security_controller",
                ))?;
                return Ok(YukiResult::denied(
                    request_id,
                    correlation_id,
                    format!("Ação rejeitada por segurança: {}", reason),
                ));
            }
        };

        // 6. Capability Execution
        let attempt_id = AttemptId::new();
        let exec_req = ExecutionRequest {
            operation_id: operation_id.clone(),
            attempt_id: attempt_id.clone(),
            capability_id: proposal.capability_id.clone(),
            authorization_token: auth_token,
            input: proposal.parameters.clone(),
        };

        self.event_store.record(AuditEvent::new(
            EventType::OperationDispatched,
            correlation_id.clone(),
            CausationId::new(operation_id.0.clone()),
            serde_json::json!({
                "operation_id": operation_id.0,
                "attempt_id": attempt_id.0
            }),
            "executor",
        ))?;

        let exec_result = self.executor.execute(
            &exec_req,
            &self.capability_registry,
            &self.security_controller,
        )?;

        // 7. Verification Layer
        let evidences = match &exec_result.state {
            OperationState::Completed => {
                if let Some(output) = &exec_result.output {
                    vec![Evidence {
                        evidence_id: EvidenceId::new(),
                        source: "executor_output".to_string(),
                        data: output.clone(),
                        observed_at: now_utc(),
                        confidence_basis: "deterministic_execution_output".to_string(),
                    }]
                } else {
                    Vec::new()
                }
            }
            _ => Vec::new(),
        };

        self.event_store.record(AuditEvent::new(
            EventType::EffectObserved,
            correlation_id.clone(),
            CausationId::new(attempt_id.0.clone()),
            serde_json::json!({
                "operation_state": format!("{:?}", exec_result.state),
                "evidence_count": evidences.len()
            }),
            "verification_engine",
        ))?;

        let verification = self
            .verifier
            .verify(&operation_id, &exec_result, &evidences);

        self.event_store.record(AuditEvent::new(
            EventType::VerificationCompleted,
            correlation_id.clone(),
            CausationId::new(operation_id.0.clone()),
            serde_json::json!({
                "verification_state": format!("{:?}", verification.verification_state),
                "effect_state": format!("{:?}", verification.observed_effect_state)
            }),
            "verification_engine",
        ))?;

        // 8. Result Production
        let output_text = if let Some(out) = &exec_result.output {
            if let Some(msg) = out.get("echoed_message").and_then(|v| v.as_str()) {
                msg.to_string()
            } else if let Some(msg) = out.get("message").and_then(|v| v.as_str()) {
                msg.to_string()
            } else if let Some(msg) = out.get("content").and_then(|v| v.as_str()) {
                msg.to_string()
            } else if let Some(s) = out.as_str() {
                s.to_string()
            } else {
                out.to_string()
            }
        } else if let Some(err) = &exec_result.error {
            format!("Erro na execução: {}", err)
        } else {
            "Operação concluída sem saída legível.".to_string()
        };

        let result = if verification.is_success() {
            YukiResult::success(
                request_id.clone(),
                correlation_id.clone(),
                Some(operation_id),
                output_text,
                Some(verification),
            )
        } else if verification.is_unknown() {
            YukiResult::unknown(
                request_id.clone(),
                correlation_id.clone(),
                Some(operation_id),
                output_text,
                Some(verification),
            )
        } else {
            YukiResult::failed(
                request_id.clone(),
                correlation_id.clone(),
                format!("Falha na verificação da operação: {}", output_text),
            )
        };

        self.event_store.record(AuditEvent::new(
            EventType::ResponseProduced,
            correlation_id,
            CausationId::new(request_id.to_string()),
            serde_json::json!({
                "status": format!("{:?}", result.status)
            }),
            "yuki_core",
        ))?;

        Ok(result)
    }

    /// Processamento síncrono padrão (compatibilidade e preservação da Foundation).
    pub fn process_input(&self, input: UserInput) -> Result<YukiResult, YukiError> {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => {
                tokio::task::block_in_place(|| handle.block_on(self.process_input_async(input)))
            }
            Err(_) => {
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .map_err(|e| YukiError::ModelError(e.to_string()))?;
                rt.block_on(self.process_input_async(input))
            }
        }
    }

    /// Asynchronous processing with explicit future timeout boundary.
    pub async fn process_input_with_timeout(
        &self,
        input: UserInput,
        timeout: std::time::Duration,
    ) -> Result<YukiResult, YukiError> {
        match tokio::time::timeout(timeout, self.process_input_async(input)).await {
            Ok(result) => result,
            Err(_) => Err(YukiError::ExecutionFailed(format!(
                "Operation exceeded execution timeout of {:?}",
                timeout
            ))),
        }
    }
}

impl Default for YukiCore {
    fn default() -> Self {
        Self::new()
    }
}
