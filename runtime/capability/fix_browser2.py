import os

path = r"c:\Users\DaRkAngeL\Desktop\cognyxos\runtime\capability\src\browser.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

# Fix struct CapabilityDefinition instantiations
# We have:
# CapabilityDefinition {
#     capability_id: "browser.open".into(),
#     name: "Open Browser".into(),
#     version: crate::model::CapabilityVersion::v1(),
#     description: "Open a URL in the browser".into(),
#     input_schema: json!({"url": "string"}),
#     output_schema: json!({"session_id": "string"}),
#     metadata: CapabilityMetadata {
#         required_permissions: vec!["browser.open".into()],
#         required_resources: vec![],
#         supported_runtimes: vec![CapabilityRuntime::Windows, CapabilityRuntime::Linux, CapabilityRuntime::MacOS],
#         security_level: SecurityLevel::Low,
#         risk_level: RiskLevel::Low,
#         idempotency: Idempotency::NonIdempotent,
#         timeout_ms: 30000,
#         audit_policy: AuditPolicy::Metadata,
#     },
#     deprecated: false,
# }
# But the file probably just has:
# CapabilityDefinition {
#     id: "...",
#     name: "...",
#     version: "1.0.0".into(),
#     description: "...",
#     parameters: json!({}),
#     permissions_required: vec![],
# }
# Let's just use regex to replace it with CapabilityDefinition::basic

import re

def replacer(match):
    id_str = match.group(1)
    desc_str = match.group(2)
    perms_str = match.group(3)
    
    return f"""CapabilityDefinition {{
    capability_id: "{id_str}".into(),
    name: "{id_str}".into(),
    version: crate::model::CapabilityVersion::v1(),
    description: "{desc_str}".into(),
    input_schema: serde_json::Value::Object(Default::default()),
    output_schema: serde_json::Value::Object(Default::default()),
    metadata: crate::model::CapabilityMetadata {{
        required_permissions: {perms_str},
        required_resources: vec![],
        supported_runtimes: vec![crate::model::CapabilityRuntime::Windows, crate::model::CapabilityRuntime::Linux, crate::model::CapabilityRuntime::MacOS],
        security_level: crate::model::SecurityLevel::Low,
        risk_level: crate::model::RiskLevel::Low,
        idempotency: crate::model::Idempotency::NonIdempotent,
        timeout_ms: 30000,
        audit_policy: crate::model::AuditPolicy::Metadata,
    }},
    deprecated: false,
}}"""

content = re.sub(
    r'CapabilityDefinition\s*\{\s*id:\s*"([^"]+)".into\(\),\s*name:\s*"[^"]+".into\(\),\s*version:\s*"1\.0\.0"\.into\(\),\s*description:\s*"([^"]+)".into\(\),\s*parameters:\s*json!\(\{[^}]*\}\),\s*permissions_required:\s*(vec!\[[^\]]*\]),\s*\}',
    replacer,
    content
)

content = content.replace("context.request.arguments", "context.request.input")
content = content.replace("context.request.capability", "context.request.capability_id")
content = content.replace("CapabilityErrorCode::ProviderError", "CapabilityErrorCode::Internal")
content = content.replace("CapabilityErrorCode::ProviderNotFound", "CapabilityErrorCode::ProviderUnavailable")

with open(path, "w", encoding="utf-8") as f:
    f.write(content)
