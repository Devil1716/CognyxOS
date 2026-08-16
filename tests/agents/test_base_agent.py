from core.agents import AgentState, BaseAgent


def test_agent_has_required_identity():
    agent = BaseAgent(goal_id="goal-123")

    assert agent.agent_id
    assert agent.run_id
    assert agent.goal_id == "goal-123"
    assert agent.state == AgentState.CREATED


def test_agent_accepts_explicit_identity():
    agent = BaseAgent(
        goal_id="goal-123",
        agent_id="agent-123",
        run_id="run-123",
    )

    assert agent.agent_id == "agent-123"
    assert agent.run_id == "run-123"
    assert agent.goal_id == "goal-123"


def test_agent_initialization():
    agent = BaseAgent(goal_id="goal-123")

    agent.initialize("correlation-1")

    assert agent.state == AgentState.IDLE
    assert len(agent.lifecycle.history) == 2


def test_agent_execution_flow():
    agent = BaseAgent(goal_id="goal-123")
    correlation_id = "run-123"

    agent.initialize(correlation_id)
    agent.start_planning(correlation_id)
    agent.begin_reasoning(correlation_id)
    agent.begin_execution(correlation_id)
    agent.begin_observation(correlation_id)
    agent.return_to_idle(correlation_id)

    assert agent.state == AgentState.IDLE


def test_agent_can_complete():
    agent = BaseAgent(goal_id="goal-123")

    agent.initialize("complete-test")
    agent.complete("complete-test")

    assert agent.state == AgentState.COMPLETED


def test_agent_can_fail():
    agent = BaseAgent(goal_id="goal-123")
    correlation_id = "failure-test"

    agent.initialize(correlation_id)
    agent.start_planning(correlation_id)
    agent.begin_reasoning(correlation_id)
    agent.begin_execution(correlation_id)

    agent.fail(
        correlation_id,
        reason="Execution failed.",
    )

    assert agent.state == AgentState.FAILED
def test_agent_can_recover():
    agent = BaseAgent(goal_id="goal-123")

    agent.initialize("recover-test")
    agent.lifecycle.transition(
        AgentState.PAUSED,
        reason="Pause",
        correlation_id="recover-test",
    )

    agent.recover("recover-test")

    assert agent.state == AgentState.IDLE


def test_completed_agent_can_shutdown():
    agent = BaseAgent(goal_id="goal-123")

    agent.initialize("shutdown-test")
    agent.complete("shutdown-test")
    agent.shutdown("shutdown-test")

    assert agent.state == AgentState.SHUTDOWN