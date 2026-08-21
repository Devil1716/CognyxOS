"""Capability adapter: translate real capability calls into run_task form.

Adapts TaskExecutor's injected ``run_task: Callable[[Task], bool]`` signature
to real OllamaProvider and FilesystemReadTool invocations. TaskExecutor owns
the task lifecycle (pending -> ready -> running -> completed|failed); this
thin layer supplies the actual "running" work.

``task.kind`` selects the operation, and ``task.capabilities`` must declare
that operation for it to run. True means the capability call succeeded. False
means a genuine capability failure: an OllamaProviderError from the provider,
a ReadToolError from the filesystem tool, or an unrecognized/undeclared
operation for the task.

Programming errors (a malformed Task, a missing required input field, a
TypeError) are deliberately NOT caught here; they propagate to TaskExecutor's
existing exception handling.
"""

from core.models.ollama_provider import OllamaProvider, OllamaProviderError
from core.planner.task_graph import Task
from core.tools.filesystem_read import FilesystemReadTool, ReadToolError

# task.kind -> capability identifiers a task must declare for the operation.
_KIND_REQUIRED_CAPABILITIES: dict[str, tuple[str, ...]] = {
    "model.inference": ("chat",),
    "filesystem.read": ("org.cognyx.filesystem.read",),
}


class CapabilityAdapter:
    """Makes a real provider/tool invocation look like TaskExecutor.run_task."""

    def __init__(
        self,
        model_provider: OllamaProvider,
        filesystem_tool: FilesystemReadTool,
    ) -> None:
        self.model_provider = model_provider
        self.filesystem_tool = filesystem_tool

    def __call__(self, task: Task) -> bool:
        """Run the task's capability call; True on success, False on failure."""
        required = _KIND_REQUIRED_CAPABILITIES.get(task.kind)
        if required is None:
            # An unrecognized/unsupported task kind.
            return False
        if not set(task.capabilities) & set(required):
            # A recognized kind, but this task did not declare the operation.
            return False

        if task.kind == "model.inference":
            return self._model_inference(task)
        return self._filesystem_read(task)

    def _model_inference(self, task: Task) -> bool:
        try:
            self.model_provider.inference(
                request_id=task.input["request_id"],
                prompt=task.input["prompt"],
                capability=_KIND_REQUIRED_CAPABILITIES["model.inference"][0],
                correlation_id=task.task_id,
            )
        except OllamaProviderError:
            return False
        return True

    def _filesystem_read(self, task: Task) -> bool:
        try:
            self.filesystem_tool.execute(task.input)
        except ReadToolError:
            return False
        return True