"""End-to-end observe-plan-act Agent integration test.

Proves that the three independently built pieces compose correctly in one
real loop without a real model or a new executor module:

- BaseAgent (core/agents/base_agent.py) owns the lifecycle state machine.
- OllamaProvider (core/models/ollama_provider.py) provides the "thinking"
  step; its HTTP call is faked through the constructor-injected json_client
  seam, so no real Ollama or network is used.
- FilesystemReadTool (core/tools/filesystem_read.py) provides the "acting"
  step against a real temporary directory.

The loop is driven by calling the lifecycle and provider/tool methods
directly. Once the task graph contract is implemented, a real
executor/planner will replace this manual step-calling; nothing in this test
depends on such a module existing.
"""

import json

from cognyx_runtime.events import EventBus

from core.agents import AgentState, BaseAgent
from core.models.ollama_provider import OllamaProvider
from core.tools.filesystem_read import FilesystemReadTool


def test_agent_end_to_end_observe_plan_act_loop(tmp_path):
    greeting = "Hello from temp"
    prompt = "Which file should I read to complete my goal?"
    correlation_id = "run-1"

    # --- one shared real event bus the whole loop publishes onto ---------
    events = []
    bus = EventBus()
    bus.subscribe(events.append)

    # --- a real lifecycle-aware agent -----------------------------------
    agent = BaseAgent(
        goal_id="integration: read the greeting file",
        event_bus=bus,
        publisher="agent-host",
    )

    # --- a real model provider whose HTTP call is faked -----------------
    def fake_ollama(method, url, payload=None, timeout_seconds=5.0):
        assert payload is not None and payload["prompt"] == prompt
        return {"model": "llama3.2", "response": "notes.txt", "done": True}

    provider = OllamaProvider(
        model_id="llama3.2",
        event_bus=bus,
        publisher="model-provider-host",
        json_client=fake_ollama,
    )

    # --- a real filesystem tool bound to a real temp directory ----------
    target = tmp_path / "notes.txt"
    target.write_text(greeting, encoding="utf-8")
    tool = FilesystemReadTool(tmp_path, event_bus=bus, publisher="tool-host")

    # 1. create and initialize
    agent.initialize(correlation_id)

    # 2. plan, then "think" using the model provider
    agent.start_planning(correlation_id)
    agent.begin_reasoning(correlation_id)
    decision = provider.inference(
        request_id="think-1",
        prompt=prompt,
        correlation_id=correlation_id,
    )
    plan = decision["content"]

    # 3. "act" using the model's output as the tool input
    agent.begin_execution(correlation_id)
    observation = tool.execute({"path": plan})

    # 4. observe the tool result, then return to idle
    agent.begin_observation(correlation_id)
    agent.return_to_idle(correlation_id)

    # 5. complete
    agent.complete(correlation_id)

    # --- assertions ------------------------------------------------------
    assert agent.state == AgentState.COMPLETED

    history = [record.current for record in agent.lifecycle.history]
    assert history == [
        AgentState.INITIALIZING,
        AgentState.IDLE,
        AgentState.PLANNING,
        AgentState.REASONING,
        AgentState.EXECUTING,
        AgentState.OBSERVING,
        AgentState.IDLE,
        AgentState.COMPLETED,
    ]

    # Exactly three events, in order; inference itself never published one.
    assert [event.event_type for event in events] == [
        "org.cognyx.agent.started",
        "org.cognyx.tool.executed",
        "org.cognyx.agent.finished",
    ]
    finished = [event for event in events if event.event_type == "org.cognyx.agent.finished"]
    assert len(finished) == 1
    assert finished[0].payload["outcome"] == "completed"

    # No raw prompt or file content ever reached an event payload.
    serialized = json.dumps([event.payload for event in events])
    assert prompt not in serialized
    assert greeting not in serialized

    # The tool's output reached the test in memory, on the happy path.
    assert plan == "notes.txt"
    assert decision["request_id"] == "think-1"
    assert observation["content"] == greeting
    assert observation["path"].endswith("notes.txt")