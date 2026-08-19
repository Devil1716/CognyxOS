"""Planner (first, deliberately narrow slice): structured steps -> TaskGraph.

CONTRACT GAP: there is no formal Planner contract in this repository. The
only mention is the diagram in docs/contracts/lifecycles.md ("Goal ->
Planner -> DAG -> dependency resolution -> scheduler -> execution ->
verification -> completion"); no detailed API specification exists. This
module is therefore a minimal interpretation, NOT a verified planner
specification.

Scope: it does NOT parse natural-language goals (an LLM turning "read this
file and summarize it" into a step list is real future work and is NOT built
here). This slice takes a STRUCTURED list of Step inputs
(label, kind, capabilities, input, depends_on) and turns it into a valid,
ready-to-run TaskGraph under a narrow-but-genuine planner definition:
validating capabilities against the caller-declared set, validating and
resolving dependency labels, detecting cycles, and generating task_ids.

Task-id mapping (deterministic): task_id == step.label exactly. Rebuilding
the same structured steps produces the same task IDs; no random UUIDs are
used. Duplicate labels are rejected because they would produce ambiguous
task identity.
"""

from dataclasses import dataclass, field

from core.planner.task_graph import Task, TaskGraph


class PlannerError(ValueError):
    """Raised when structured steps cannot become a valid task graph."""


@dataclass(frozen=True, slots=True)
class Step:
    """One structured planner input step.

    ``depends_on`` lists OTHER STEPS' LABELS - not task_ids. The planner
    turns labels into task_ids (task_id == label) while building the graph.
    """

    label: str
    kind: str
    capabilities: tuple[str, ...] = ()
    input: dict[str, object] = field(default_factory=dict)
    depends_on: tuple[str, ...] = ()


class Planner:
    """Builds valid, ready-to-run TaskGraphs from structured steps."""

    def build_graph(
        self,
        steps: list[Step],
        available_capabilities: set[str],
    ) -> TaskGraph:
        """Validate and build a TaskGraph from structured steps.

        All validation happens BEFORE any task is added to the graph. Raises
        PlannerError for: duplicate labels, capabilities that are not in
        ``available_capabilities``, depends_on labels that name no step in
        the list, or circular dependencies (which would otherwise make
        TaskGraph.ready_tasks() silently never return those tasks).
        """
        labels = [step.label for step in steps]
        if len(set(labels)) != len(labels):
            duplicates = sorted(
                {label for label in labels if labels.count(label) > 1}
            )
            raise PlannerError(
                "Duplicate step labels are not allowed: "
                + ", ".join(f"'{label}'" for label in duplicates)
            )

        by_label = {step.label: step for step in steps}

        missing_capabilities = sorted(
            {
                capability
                for step in steps
                for capability in step.capabilities
                if capability not in available_capabilities
            }
        )
        if missing_capabilities:
            raise PlannerError(
                "Capabilities not available to the planner: "
                + ", ".join(f"'{cap}'" for cap in missing_capabilities)
            )

        unknown_dependencies = sorted(
            {
                dependency
                for step in steps
                for dependency in step.depends_on
                if dependency not in by_label
            }
        )
        if unknown_dependencies:
            raise PlannerError(
                "Steps depend on labels that are not present: "
                + ", ".join(f"'{dep}'" for dep in unknown_dependencies)
            )

        self._assert_acyclic(steps, by_label)

        graph = TaskGraph()
        for step in steps:
            graph.add_task(
                Task(
                    task_id=step.label,
                    kind=step.kind,
                    input=dict(step.input),
                    capabilities=tuple(step.capabilities),
                    dependencies=tuple(step.depends_on),
                )
            )
        return graph

    @staticmethod
    def _assert_acyclic(
        steps: list[Step],
        by_label: dict[str, Step],
    ) -> None:
        """Raise PlannerError if any dependency cycle exists among steps."""

        visiting: set[str] = set()
        visited: set[str] = set()

        def visit(label: str) -> None:
            if label in visited:
                return
            if label in visiting:
                raise PlannerError(
                    f"Circular dependency detected involving step '{label}'."
                )
            visiting.add(label)
            for dependency in by_label[label].depends_on:
                visit(dependency)
            visiting.remove(label)
            visited.add(label)

        for step in steps:
            visit(step.label)