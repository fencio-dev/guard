# app/policy_templates.py
from pydantic import BaseModel

class PolicyTemplate(BaseModel):
    id: str
    name: str
    description: str
    template_text: str
    category: str
    example_customizations: list[str]

POLICY_TEMPLATES = [
    PolicyTemplate(
        id="database_read_only",
        name="Database Read-Only Access",
        description="Allow agent to read from databases without write permissions",
        template_text="Allow reading from databases",
        category="database",
        example_customizations=[
            "only from analytics_db",
            "only public data",
            "excluding PII fields"
        ]
    ),
    PolicyTemplate(
        id="database_write_access",
        name="Database Write Access",
        description="Allow agent to write to databases",
        template_text="Allow writing to databases",
        category="database",
        example_customizations=[
            "only to staging databases",
            "only public data",
            "single records only"
        ]
    ),
    PolicyTemplate(
        id="file_export",
        name="File Export Capabilities",
        description="Allow agent to export data to files",
        template_text="Allow exporting data to files",
        category="file",
        example_customizations=[
            "only CSV and JSON formats",
            "only public data",
            "maximum 1000 records at a time"
        ]
    ),
    PolicyTemplate(
        id="api_read_access",
        name="API Read Access",
        description="Allow agent to call external APIs for reading data",
        template_text="Allow calling external APIs for reading data",
        category="api",
        example_customizations=[
            "only public APIs",
            "excluding payment APIs",
            "with authentication required"
        ]
    ),
    PolicyTemplate(
        id="unrestricted",
        name="Unrestricted Access",
        description="Allow agent full access to all operations",
        template_text="Allow all operations",
        category="general",
        example_customizations=[
            "with authentication required",
            "excluding production databases"
        ]
    )
]

def get_template_by_id(template_id: str) -> PolicyTemplate | None:
    """Get template by ID."""
    return next((t for t in POLICY_TEMPLATES if t.id == template_id), None)

def get_templates_by_category(category: str) -> list[PolicyTemplate]:
    """Get templates by category."""
    return [t for t in POLICY_TEMPLATES if t.category == category]
