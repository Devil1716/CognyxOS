import pytest

from core.agents.lifecycle import (
    AgentLifecycleCoordinator,
    AgentLifecycleError,
    AgentLifecycleRecord,
    AgentState,
)


def test_initial_state_is_created():
    coordinator = AgentLifecycleCoordinator()

    assert coordinator.state == AgentState.CREATED
    assert coordinator.history == []


def test_valid_transition_records_metadata():
    coordinator = AgentLifecycleCoordinator()

    record = coordinator.transition(
        AgentState.INITIALIZING,
        reason="Agent startup",
        correlation_id="test-123",
    )

    assert coordinator.state == AgentState.INITIALIZING
    assert record.previous == AgentState.CREATED
    assert record.current == AgentState.INITIALIZING
    assert record.reason == "Agent startup"
    assert record.correlation_id == "test-123"
    assert record.timestamp


def test_missing_reason_is_rejected():
    coordinator = AgentLifecycleCoordinator()

    with pytest.raises(AgentLifecycleError):
        coordinator.transition(
            AgentState.INITIALIZING,
            reason="",
            correlation_id="test-123",
        )


def test_missing_correlation_id_is_rejected():
    coordinator = AgentLifecycleCoordinator()

    with pytest.raises(AgentLifecycleError):
        coordinator.transition(
            AgentState.INITIALIZING,
            reason="Agent startup",
            correlation_id="",
        )


def test_invalid_transition_is_rejected():
    coordinator = AgentLifecycleCoordinator()

    with pytest.raises(AgentLifecycleError):
        coordinator.transition(
            AgentState.EXECUTING,
            reason="Invalid transition",
            correlation_id="test-456",
        )


def test_normal_agent_flow():
    coordinator = AgentLifecycleCoordinator()

    transitions = [
        AgentState.INITIALIZING,
        AgentState.IDLE,
        AgentState.PLANNING,
        AgentState.REASONING,
        AgentState.EXECUTING,
        AgentState.OBSERVING,
        AgentState.IDLE,
    ]

    for state in transitions:
        coordinator.transition(
            state,
            reason=f"Transition to {state}",
            correlation_id="test-flow",
        )

    assert coordinator.state == AgentState.IDLE
    assert len(coordinator.history) == len(transitions)


def test_checkpoint_happens_before_state_change():
    checkpoints: list[AgentLifecycleRecord] = []

    def checkpoint(record: AgentLifecycleRecord) -> None:
        checkpoints.append(record)

    coordinator = AgentLifecycleCoordinator(
        checkpoint_writer=checkpoint,
    )

    coordinator.transition(
        AgentState.INITIALIZING,
        reason="Initialize",
        correlation_id="checkpoint-test",
    )

    assert len(checkpoints) == 1
    assert checkpoints[0].current == AgentState.INITIALIZING
    assert coordinator.state == AgentState.INITIALIZING


def test_failed_checkpoint_does_not_change_state():
    def checkpoint(record: AgentLifecycleRecord) -> None:
        raise RuntimeError("checkpoint failed")

    coordinator = AgentLifecycleCoordinator(
        checkpoint_writer=checkpoint,
    )

    with pytest.raises(RuntimeError):
        coordinator.transition(
            AgentState.INITIALIZING,
            reason="Initialize",
            correlation_id="checkpoint-test",
        )

    assert coordinator.state == AgentState.CREATED
    assert coordinator.history == []


def test_cancellation_enters_paused_after_compensation():
    compensation_called: list[bool] = []

    def compensate() -> None:
        compensation_called.append(True)

    coordinator = AgentLifecycleCoordinator()

    coordinator.transition(
        AgentState.INITIALIZING,
        reason="Initialize",
        correlation_id="cancel-test",
    )
    coordinator.transition(
        AgentState.IDLE,
        reason="Ready",
        correlation_id="cancel-test",
    )
    coordinator.transition(
        AgentState.PLANNING,
        reason="Plan",
        correlation_id="cancel-test",
    )

    record = coordinator.cancel(
        reason="User cancelled",
        correlation_id="cancel-test",
        compensation=compensate,
    )

    assert compensation_called == [True]
    assert record.current == AgentState.PAUSED
    assert coordinator.state == AgentState.PAUSED


def test_terminal_agent_cannot_be_cancelled():
    coordinator = AgentLifecycleCoordinator()

    coordinator.transition(
        AgentState.INITIALIZING,
        reason="Initialize",
        correlation_id="terminal-test",
    )
    coordinator.transition(
        AgentState.IDLE,
        reason="Ready",
        correlation_id="terminal-test",
    )
    coordinator.transition(
        AgentState.COMPLETED,
        reason="Complete",
        correlation_id="terminal-test",
    )

    with pytest.raises(AgentLifecycleError):
        coordinator.cancel(
            reason="Too late",
            correlation_id="terminal-test",
        )