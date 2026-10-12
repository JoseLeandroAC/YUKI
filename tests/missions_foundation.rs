use std::sync::Arc;
use yuki::capabilities::registry::CapabilityRegistry;
use yuki::contracts::missions::{
    ApprovalStatus, MissionError, MissionExecutionPolicy, MissionPlan, MissionStatus, MissionStep,
    MissionStepStatus,
};
use yuki::missions::approval::MissionApprovalManager;
use yuki::missions::planner::MissionPlanner;
use yuki::missions::runner::MissionRunner;
use yuki::missions::store::SqliteMissionStore;
use yuki::verification::verifier::Verifier;

#[test]
fn test_mission_state_transitions() {
    let mut mission = yuki::contracts::missions::Mission::new(
        "m-01",
        "Pesquisa",
        "Objetivo de teste",
        "operator-1",
        "corr-1",
        MissionExecutionPolicy::default(),
    );
    assert_eq!(mission.status, MissionStatus::Planned);

    // Planned -> InProgress
    assert!(mission.transition_to(MissionStatus::InProgress).is_ok());

    // InProgress -> AwaitingApproval
    assert!(mission
        .transition_to(MissionStatus::AwaitingApproval)
        .is_ok());

    // AwaitingApproval -> InProgress
    assert!(mission.transition_to(MissionStatus::InProgress).is_ok());

    // InProgress -> Paused
    assert!(mission.transition_to(MissionStatus::Paused).is_ok());

    // Paused -> InProgress
    assert!(mission.transition_to(MissionStatus::InProgress).is_ok());

    // InProgress -> Completed
    assert!(mission.transition_to(MissionStatus::Completed).is_ok());
    assert!(mission.completed_at.is_some());

    // Estado terminal não pode transitar para outro estado
    assert!(mission.transition_to(MissionStatus::InProgress).is_err());
}

#[test]
fn test_mission_plan_acyclic_and_cycle_detection() {
    let policy = MissionExecutionPolicy::default();

    // 1. Grafo linear válido: 1 -> 2 -> 3
    let s1 = MissionStep::new(
        "s1",
        "m",
        1,
        "passo 1",
        None,
        serde_json::json!({}),
        false,
        vec![],
    );
    let s2 = MissionStep::new(
        "s2",
        "m",
        2,
        "passo 2",
        None,
        serde_json::json!({}),
        false,
        vec![1],
    );
    let s3 = MissionStep::new(
        "s3",
        "m",
        3,
        "passo 3",
        None,
        serde_json::json!({}),
        false,
        vec![2],
    );
    let plan = MissionPlan::new(vec![s1, s2, s3]);
    assert!(plan.validate(&policy).is_ok());

    // 2. Auto-dependência direta (passo 1 depende de si mesmo)
    let bad1 = MissionStep::new(
        "b1",
        "m",
        1,
        "auto",
        None,
        serde_json::json!({}),
        false,
        vec![1],
    );
    let bad_plan1 = MissionPlan::new(vec![bad1]);
    assert!(matches!(
        bad_plan1.validate(&policy),
        Err(MissionError::CircularDependencyDetected(_))
    ));

    // 3. Ciclo indireto: 1 -> 2 -> 1
    let c1 = MissionStep::new(
        "c1",
        "m",
        1,
        "p1",
        None,
        serde_json::json!({}),
        false,
        vec![2],
    );
    let c2 = MissionStep::new(
        "c2",
        "m",
        2,
        "p2",
        None,
        serde_json::json!({}),
        false,
        vec![1],
    );
    let cycle_plan = MissionPlan::new(vec![c1, c2]);
    assert!(matches!(
        cycle_plan.validate(&policy),
        Err(MissionError::CircularDependencyDetected(_))
    ));

    // 4. Excesso de passos
    let mut many_steps = Vec::new();
    for i in 1..=20 {
        many_steps.push(MissionStep::new(
            format!("s{}", i),
            "m",
            i,
            "p",
            None,
            serde_json::json!({}),
            false,
            vec![],
        ));
    }
    let strict_policy = MissionExecutionPolicy {
        max_steps: 10,
        ..Default::default()
    };
    let big_plan = MissionPlan::new(many_steps);
    assert!(matches!(
        big_plan.validate(&strict_policy),
        Err(MissionError::StepLimitExceeded { .. })
    ));
}

#[test]
fn test_mission_sqlite_persistence_and_recovery() {
    let store = SqliteMissionStore::open_in_memory().unwrap();
    let planner = MissionPlanner::default();

    let plan = MissionPlanner::build_research_mission_plan("m-test-recovery", "rust", "out.txt");
    let mission = planner
        .create_mission(
            "m-test-recovery",
            "Missão de Teste",
            "Pesquisar e gravar",
            "operator-jose",
            "corr-123",
            plan,
            None,
        )
        .unwrap();

    // Persiste missão e passos
    store.create_mission(&mission).unwrap();
    for step in &mission.steps {
        store.save_step(step).unwrap();
    }

    // Altera passo 1 para Running para simular crash durante execução
    store
        .update_step_status(
            "m-test-recovery_step_1",
            MissionStepStatus::Running,
            None,
            None,
        )
        .unwrap();
    store
        .update_mission_status("m-test-recovery", MissionStatus::InProgress, None)
        .unwrap();

    // Reconciliação na reinicialização
    let recovered = store.reconcile_and_recover_on_startup().unwrap();
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].status, MissionStatus::Paused);
    assert_eq!(recovered[0].steps[0].status, MissionStepStatus::Pending);
}

#[test]
fn test_mission_human_approval_protocol_strictness() {
    let store = Arc::new(SqliteMissionStore::open_in_memory().unwrap());
    let approval_mgr = MissionApprovalManager::new(store.clone(), None);

    let step = MissionStep::new(
        "step_mutate",
        "m_auth",
        1,
        "Ação externa mutadora",
        Some("system.echo".to_string()),
        serde_json::json!({ "command": "format_disk" }),
        true,
        vec![],
    );

    // 1. Cria solicitação
    let approval = approval_mgr
        .request_approval("m_auth", &step, "operator-1", 60)
        .unwrap();
    assert_eq!(approval.status, ApprovalStatus::Pending);

    // 2. Tentar validar antes de conceder -> Falha
    let mut step_with_id = step.clone();
    step_with_id.approval_id = Some(approval.id.clone());
    let verify_res = approval_mgr.verify_step_approval(&step_with_id, "operator-1");
    assert!(verify_res.is_err());

    // 3. Operador incorreto tenta aprovar -> Falha
    let wrong_op_res = approval_mgr.grant_approval(&approval.id, "hacker");
    assert!(matches!(
        wrong_op_res,
        Err(MissionError::ApprovalMismatch(_))
    ));

    // 4. Operador legítimo aprova
    let granted = approval_mgr
        .grant_approval(&approval.id, "operator-1")
        .unwrap();
    assert_eq!(granted.status, ApprovalStatus::Granted);

    // 5. Verificação com parâmetros originais -> Sucesso
    assert!(approval_mgr
        .verify_step_approval(&step_with_id, "operator-1")
        .is_ok());

    // 6. Mutação adversarial de parâmetros após aprovação -> Rejeita imediatamente!
    let mut tampered_step = step_with_id.clone();
    tampered_step.parameters = serde_json::json!({ "command": "delete_everything_else" });
    let tamper_res = approval_mgr.verify_step_approval(&tampered_step, "operator-1");
    assert!(matches!(tamper_res, Err(MissionError::ApprovalMismatch(_))));
}

#[tokio::test]
async fn test_mission_runner_end_to_end_research_mission() {
    let store = Arc::new(SqliteMissionStore::open_in_memory().unwrap());
    let approval_mgr = Arc::new(MissionApprovalManager::new(store.clone(), None));
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let verifier = Arc::new(Verifier::new());

    let runner = MissionRunner::new(
        store.clone(),
        approval_mgr.clone(),
        capability_registry,
        verifier,
        None,
    );

    let planner = MissionPlanner::default();
    let plan = MissionPlanner::build_research_mission_plan(
        "m_research_e2e",
        "Whisper STT",
        "relatorio.md",
    );

    let mission = planner
        .create_mission(
            "m_research_e2e",
            "Pesquisa de Reconhecimento de Voz",
            "Levantar fontes de STT local",
            "operator-jose",
            "corr-999",
            plan,
            None,
        )
        .unwrap();

    // Persiste a missão
    store.create_mission(&mission).unwrap();
    for step in &mission.steps {
        store.save_step(step).unwrap();
    }

    // 1. Executa primeira fase (passos 1 e 2 de leitura/síntese são executados automaticamente;
    // passo 3 exige aprovação e pausa a missão no portão humano)
    let mission_state_1 = runner.run_mission(&mission.id).await.unwrap();
    assert_eq!(mission_state_1.status, MissionStatus::AwaitingApproval);

    let steps = store.get_steps_for_mission(&mission.id).unwrap();
    assert_eq!(steps[0].status, MissionStepStatus::Completed);
    assert_eq!(steps[1].status, MissionStepStatus::Completed);
    assert_eq!(steps[2].status, MissionStepStatus::WaitingApproval);
    assert!(steps[2].approval_id.is_some());

    let app_id = steps[2].approval_id.as_ref().unwrap();

    // 2. Operador concede aprovação formal para a etapa 3
    approval_mgr
        .grant_approval(app_id, "operator-jose")
        .unwrap();

    // 3. Retoma execução da missão após aprovação humana
    let final_mission = runner.run_mission(&mission.id).await.unwrap();
    assert_eq!(final_mission.status, MissionStatus::Completed);
    assert!(final_mission.completed_at.is_some());

    let final_steps = store.get_steps_for_mission(&mission.id).unwrap();
    assert_eq!(final_steps[2].status, MissionStepStatus::Completed);

    // 4. Verifica que checkpoint foi gravado para o último passo
    let latest_cp = store.get_latest_checkpoint(&mission.id).unwrap();
    assert!(latest_cp.is_some());
    assert_eq!(latest_cp.unwrap().step_number, 3);
}

#[tokio::test]
async fn test_mission_pause_resume_cancel() {
    let store = Arc::new(SqliteMissionStore::open_in_memory().unwrap());
    let approval_mgr = Arc::new(MissionApprovalManager::new(store.clone(), None));
    let capability_registry = Arc::new(CapabilityRegistry::new());
    let verifier = Arc::new(Verifier::new());

    let runner = MissionRunner::new(
        store.clone(),
        approval_mgr,
        capability_registry,
        verifier,
        None,
    );

    let s1 = MissionStep::new(
        "s1",
        "m_pause",
        1,
        "step 1",
        Some("system.echo".to_string()),
        serde_json::json!({}),
        false,
        vec![],
    );
    let s2 = MissionStep::new(
        "s2",
        "m_pause",
        2,
        "step 2",
        Some("system.echo".to_string()),
        serde_json::json!({}),
        false,
        vec![1],
    );
    let plan = MissionPlan::new(vec![s1, s2]);

    let mission = MissionPlanner::default()
        .create_mission("m_pause", "Pausável", "Obj", "op", "corr", plan, None)
        .unwrap();

    store.create_mission(&mission).unwrap();
    for step in &mission.steps {
        store.save_step(step).unwrap();
    }

    // Pausar
    runner.pause_mission("m_pause").unwrap();
    let m = store.get_mission("m_pause").unwrap().unwrap();
    assert_eq!(m.status, MissionStatus::Paused);

    // Cancelar
    runner
        .cancel_mission("m_pause", "Cancelado pelo usuário")
        .unwrap();
    let m = store.get_mission("m_pause").unwrap().unwrap();
    assert_eq!(m.status, MissionStatus::Cancelled);
    assert_eq!(m.failure_reason, Some("Cancelado pelo usuário".to_string()));
}
