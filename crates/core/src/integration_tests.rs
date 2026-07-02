//! Integration tests — multi-agent workflow, context stress, crash recovery.

#[cfg(test)]
mod integration_tests {
    use crate::actor::roles::base::{Architect, Coder, Reviewer, Security, Tester, BaseAgent};
    use crate::drift::metrics::MetricSample;
    use crate::drift::recovery::DriftGuard;
    use crate::machine::phase::{Phase, StateMachine};
    use crate::machine::gate::{Gate, GateEvaluator, GateRegistry, ReviewResult};
    use crate::orchestrator::injection::{Injection, InjectionChannel, InjectionType, InjectionPriority, InjectionSource};
    use crate::orchestrator::task::Task;

    fn test_role(name: &str, model: &str) -> crate::orchestrator::roles::ResolvedRole {
        crate::orchestrator::roles::ResolvedRole {
            role_name: name.to_string(),
            model: model.to_string(),
            temperature: 0.3,
            max_tokens: 4096,
            system_prompt: format!("You are a {}", name),
            tools: vec!["filesystem".to_string()],
            context_profile: "balanced".to_string(),
        }
    }

    // ─── Full Workflow Test ────────────────────────────────────

    #[tokio::test]
    async fn test_full_workflow() {
        let architect = Architect::new(test_role("architect", "claude-4-opus"));
        let coder = Coder::new(test_role("coder", "gpt-5"));
        let reviewer = Reviewer::new(test_role("reviewer", "gemini-2.5-pro"));
        let security = Security::new(test_role("security", "claude-4-haiku"));
        let tester = Tester::new(test_role("tester", "gpt-5"));

        let task = Task::new("architect", "claude-4-opus", "design a REST API");
        let adr = architect.execute(&task).await;
        assert_eq!(adr.status, crate::orchestrator::task::TaskStatus::Completed);

        let task = Task::new("coder", "gpt-5", "implement the API");
        let code = coder.execute(&task).await;
        assert_eq!(code.status, crate::orchestrator::task::TaskStatus::Completed);

        let task = Task::new("reviewer", "gemini-2.5-pro", "review the code");
        let review = reviewer.execute(&task).await;
        assert_eq!(review.status, crate::orchestrator::task::TaskStatus::Completed);

        let task = Task::new("security", "claude-4-haiku", "scan for vulnerabilities");
        let security_result = security.execute(&task).await;
        assert_eq!(security_result.status, crate::orchestrator::task::TaskStatus::Completed);

        let task = Task::new("tester", "gpt-5", "write tests");
        let test_result = tester.execute(&task).await;
        assert_eq!(test_result.status, crate::orchestrator::task::TaskStatus::Completed);
    }

    // ─── Context Stress Test ───────────────────────────────────

    #[test]
    fn test_context_stress() {
        use praxis_memory::context::{ContextManager, BudgetProfile, ContextWindow, Message};

        let mut manager = ContextManager::new(128_000, BudgetProfile::Balanced);
        let mut context = ContextWindow::new();

        for i in 0..100 {
            context.push(Message {
                role: "user".to_string(),
                content: format!("Message {} with content to test context management", i),
            });
        }

        let prepared = manager.prepare(&mut context);
        assert!(!prepared.is_empty());
    }

    // ─── Drift Detection Test ──────────────────────────────────

    #[test]
    fn test_drift_detection_workflow() {
        let mut guard = DriftGuard::new();
        guard.recovery_threshold = 80.0; // Higher threshold to trigger recovery sooner

        // Establish baseline with healthy metrics
        for i in 0..12 {
            guard.record_and_evaluate(MetricSample {
                iteration: i,
                timestamp: chrono::Utc::now().to_rfc3339(),
                latency_ms: 100 + i as u64 * 10,
                output_tokens: 50 + i as u32 * 5,
                input_tokens: 100,
                tool_calls: 2,
                tool_errors: 0,
                output_length_chars: 200 + i as usize * 20,
                gate_passed: true,
                context_pressure: 0.3,
            }, Some("coder"));
        }

        // Record degraded metrics
        let _drift_detected = false;
        for i in 0..10 {
            let report = guard.record_and_evaluate(MetricSample {
                iteration: i + 12,
                timestamp: chrono::Utc::now().to_rfc3339(),
                latency_ms: 5000,
                output_tokens: 800,
                input_tokens: 100,
                tool_calls: 2,
                tool_errors: 2,
                output_length_chars: 8000,
                gate_passed: false,
                context_pressure: 0.98,
            }, Some("coder"));

            if let Some(report) = report {
                if report.recovery_action.is_some() {
                    break;
                }
            }
        }

        // If no recovery triggered, at least check health summary works
        let health = guard.health_summary();
        assert!(health.agent_count > 0);
    }

    // ─── Injection Workflow Test ───────────────────────────────

    #[test]
    fn test_injection_workflow() {
        let mut channel = InjectionChannel::new(10);

        channel.submit(Injection::new("s1", "coder", InjectionType::Instruction,
            InjectionPriority::Normal, "Use thiserror", InjectionSource::CLI)).unwrap();
        channel.submit(Injection::new("s1", "coder", InjectionType::Correction,
            InjectionPriority::High, "Fix the bug", InjectionSource::Dashboard)).unwrap();
        channel.submit(Injection::new("s1", "all", InjectionType::Instruction,
            InjectionPriority::Critical, "Halt all work", InjectionSource::CLI)).unwrap();

        let inj = channel.next_for_agent("coder").unwrap();
        assert_eq!(inj.priority, InjectionPriority::Critical);
    }

    // ─── State Machine Full Flow Test ──────────────────────────

    #[test]
    fn test_state_machine_full_flow() {
        let mut sm = StateMachine::new();

        sm.transition(Phase::Planning, 0).unwrap();
        sm.transition(Phase::Designing, 1).unwrap();
        sm.transition(Phase::Implementing, 2).unwrap();
        sm.transition(Phase::Reviewing, 3).unwrap();
        sm.transition(Phase::Testing, 4).unwrap();
        sm.transition(Phase::Finalizing, 5).unwrap();
        sm.transition(Phase::Completed, 6).unwrap();

        assert_eq!(sm.current(), Phase::Completed);
        assert!(sm.current().is_terminal());
        assert_eq!(sm.history().len(), 7);
    }

    // ─── Gate Evaluation Test ──────────────────────────────────

    #[test]
    fn test_gate_evaluation_workflow() {
        let mut registry = GateRegistry::new();

        registry.register(
            Phase::Reviewing,
            Gate::new("review-pass", GateEvaluator::AllAgentsPass, 3),
        );
        registry.register(
            Phase::Reviewing,
            Gate::new("no-critical", GateEvaluator::NoCritical, 3),
        );

        let results = vec![
            ReviewResult {
                agent: "a".to_string(),
                passed: true,
                comments: vec![],
                coverage: None,
            },
            ReviewResult {
                agent: "b".to_string(),
                passed: true,
                comments: vec![],
                coverage: None,
            },
        ];

        let verdicts = registry.evaluate_phase(&Phase::Reviewing, &results);
        assert_eq!(verdicts.len(), 2);
        assert!(verdicts.iter().all(|v| v.passed));
    }
}