import os

path = r"c:\Users\DaRkAngeL\Desktop\cognyxos\runtime\capability\src\browser.rs"
with open(path, "r", encoding="utf-8") as f:
    content = f.read()

content = content.replace("permissions_required", "metadata.required_permissions")
content = content.replace("id:", "capability_id:")
content = content.replace("parameters:", "input_schema:")
content = content.replace("context.request.arguments", "context.request.input")
content = content.replace("context.request.capability", "context.request.capability_id")
content = content.replace("CapabilityErrorCode::ProviderError", "CapabilityErrorCode::Internal")
content = content.replace("CapabilityErrorCode::ProviderNotFound", "CapabilityErrorCode::ProviderUnavailable")
content = content.replace('version: "1.0.0".into()', 'version: crate::model::CapabilityVersion::v1()')

# also handle missing field replacements if they were chained
# wait, the metadata is a struct. so I can't just do metadata.required_permissions: vec![]
# Let's write a smarter script.
