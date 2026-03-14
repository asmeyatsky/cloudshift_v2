"""CloudShift Jupyter/Notebook integration."""

from cloudshift import transform_repo, TransformConfig, SourceCloud


def analyse(path: str = ".", source_cloud: str = "any") -> None:
    """Render a visual migration manifest in the notebook."""
    from IPython.display import display, HTML

    report = transform_repo(
        path=path,
        config=TransformConfig(
            source_cloud=SourceCloud(source_cloud),
            dry_run=True,
        ),
    )
    rows = []
    for change in report.changes:
        rows.append(
            f"<tr><td>{change.file}</td>"
            f"<td>{change.patterns_matched}</td>"
            f"<td>{change.confidence:.2f}</td></tr>"
        )
    table = (
        "<table><tr><th>File</th><th>Patterns</th><th>Confidence</th></tr>"
        + "".join(rows)
        + "</table>"
    )
    display(HTML(table))


def diff(path: str = ".", source_cloud: str = "any") -> None:
    """Render a syntax-highlighted diff explorer in the notebook."""
    from IPython.display import display, HTML

    report = transform_repo(
        path=path,
        config=TransformConfig(
            source_cloud=SourceCloud(source_cloud),
            dry_run=True,
        ),
    )
    html_parts = []
    for change in report.changes:
        html_parts.append(
            f"<details><summary>{change.file} "
            f"(confidence: {change.confidence:.2f})</summary>"
            f"<pre>{change.diff}</pre></details>"
        )
    display(HTML("".join(html_parts)))
