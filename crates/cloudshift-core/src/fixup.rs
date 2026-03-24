//! Post-transform fixup pass.
//!
//! After AST-level pattern transforms replace individual API calls,
//! surrounding code may reference old API response shapes that no
//! longer exist. This module applies text-level fixups to make the
//! output actually runnable on GCP.
//!
//! Also provides DynamoDB → standard JSON marshaling (AWS AttributeValue
//! format → plain JSON) for Firestore document payloads.

use crate::domain::value_objects::Language;

// ---------------------------------------------------------------------------
// DynamoDB → standard JSON (Firestore) marshaling
// ---------------------------------------------------------------------------

/// Convert a DynamoDB AttributeValue-style JSON value to standard JSON.
/// Handles `{"S": "x"}`, `{"N": "123"}`, `{"M": {...}}`, `{"L": [...]}`, etc.
/// Used when transforming DynamoDB put_item Item payloads to Firestore .set().
pub fn dynamodb_item_to_standard_json(
    value: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use serde_json::Value;
    match value {
        Value::Object(map) if map.len() == 1 => {
            let (k, v) = map.iter().next().expect("len is 1");
            match k.as_str() {
                "S" => {
                    return Ok(Value::String(
                        v.as_str().ok_or("S must be string")?.to_string(),
                    ))
                }
                "N" => {
                    let s = v.as_str().ok_or("N must be string")?;
                    if let Ok(n) = s.parse::<i64>() {
                        return Ok(Value::Number(serde_json::Number::from(n)));
                    }
                    if let Ok(n) = s.parse::<f64>() {
                        return Ok(Value::Number(
                            serde_json::Number::from_f64(n).ok_or("invalid N")?,
                        ));
                    }
                    return Err(format!("N not a number: {}", s));
                }
                "BOOL" => return Ok(Value::Bool(v.as_bool().ok_or("BOOL must be bool")?)),
                "NULL" => return Ok(Value::Null),
                "M" => {
                    let m = v.as_object().ok_or("M must be object")?;
                    let mut out = serde_json::Map::new();
                    for (key, val) in m {
                        out.insert(key.clone(), dynamodb_item_to_standard_json(val)?);
                    }
                    return Ok(Value::Object(out));
                }
                "L" => {
                    let arr = v.as_array().ok_or("L must be array")?;
                    let out: Result<Vec<_>, _> =
                        arr.iter().map(dynamodb_item_to_standard_json).collect();
                    return Ok(Value::Array(out?));
                }
                "SS" => {
                    let arr = v.as_array().ok_or("SS must be array")?;
                    let out: Vec<_> = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .collect();
                    return Ok(Value::Array(out.into_iter().map(Value::String).collect()));
                }
                "NS" => {
                    let arr = v.as_array().ok_or("NS must be array")?;
                    let mut out = Vec::with_capacity(arr.len());
                    for v in arr {
                        let s = v.as_str().ok_or("NS elements must be strings")?;
                        if let Ok(n) = s.parse::<i64>() {
                            out.push(Value::Number(serde_json::Number::from(n)));
                        } else if let Ok(n) = s.parse::<f64>() {
                            out.push(Value::Number(
                                serde_json::Number::from_f64(n).ok_or("invalid NS")?,
                            ));
                        } else {
                            return Err(format!("NS element not a number: {}", s));
                        }
                    }
                    return Ok(Value::Array(out));
                }
                "B" | "BS" => {
                    // Keep binary as base64 string for Firestore (no native binary type)
                    if k == "B" {
                        return Ok(Value::String(
                            v.as_str().ok_or("B must be string")?.to_string(),
                        ));
                    }
                    let arr = v.as_array().ok_or("BS must be array")?;
                    let out: Vec<_> = arr
                        .iter()
                        .filter_map(|v| v.as_str())
                        .map(String::from)
                        .map(Value::String)
                        .collect();
                    return Ok(Value::Array(out));
                }
                _ => {}
            }
        }
        _ => {}
    }
    // Not a DynamoDB wrapper — return as-is (e.g. already plain JSON)
    Ok(value.clone())
}

/// Convert a JSON object (DynamoDB Item) where each value may be AttributeValue format.
pub fn dynamodb_item_map_to_standard_json(
    value: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let map = value.as_object().ok_or("expected object")?;
    let mut out = serde_json::Map::new();
    for (k, v) in map {
        out.insert(k.clone(), dynamodb_item_to_standard_json(v)?);
    }
    Ok(serde_json::Value::Object(out))
}

/// Parse a string as JSON and convert DynamoDB AttributeValue format to standard JSON.
/// Returns the JSON string for embedding in generated code (e.g. Firestore .set({...})).
pub fn dynamodb_item_json_string_to_standard(item_json: &str) -> Result<String, String> {
    let value: serde_json::Value = serde_json::from_str(item_json).map_err(|e| e.to_string())?;
    let standard = if value.is_object() {
        dynamodb_item_map_to_standard_json(&value)?
    } else {
        dynamodb_item_to_standard_json(&value)?
    };
    serde_json::to_string(&standard).map_err(|e| e.to_string())
}

/// Apply post-transform fixups to make the output runnable.
/// These are applied AFTER all pattern transforms and import management.
pub fn apply_fixups(source: &str, language: Language) -> String {
    match language {
        Language::Python => apply_python_fixups(source),
        Language::TypeScript | Language::JavaScript => apply_typescript_fixups(source),
        _ => source.to_string(),
    }
}

fn apply_python_fixups(source: &str) -> String {
    let mut result = source.to_string();

    // === S3 get_object -> download_as_bytes fixups ===
    // Pattern: after download_as_bytes(), the old code does response['Body'].read()
    // which is now unnecessary because download_as_bytes() returns bytes directly.
    //
    // Before (broken):
    //   response = storage.Client().bucket(X).blob(Y).download_as_bytes()
    //   content = response['Body'].read().decode('utf-8')
    //
    // After (fixed):
    //   content = storage.Client().bucket(X).blob(Y).download_as_bytes().decode('utf-8')
    result = fix_download_as_bytes_pattern(&result);

    // === S3 list_objects_v2 -> list_blobs fixups ===
    // Pattern: after list_blobs(), code does response.get('Contents', [])
    // and accesses obj['Key']. list_blobs returns Blob objects with .name
    //
    // Before (broken):
    //   response = list(storage.Client().bucket(X).list_blobs(prefix=Y))
    //   return [obj['Key'] for obj in response.get('Contents', [])]
    //
    // After (fixed):
    //   blobs = list(storage.Client().bucket(X).list_blobs(prefix=Y))
    //   return [blob.name for blob in blobs]
    result = fix_list_blobs_pattern(&result);

    // === Exception fixups ===
    // s3.exceptions.ClientError -> google.cloud.exceptions.NotFound
    // boto3 exceptions -> google.cloud exceptions
    //
    // We handle specific client-variable patterns first (more specific), then
    // fall through to the generic ClientError replacement only in except clauses.
    result = fix_exception_references(&result);

    // === URI scheme fixups ===
    // Rewrite s3:// → gs:// only when this file no longer uses an S3 boto3 client
    // (avoids corrupting intentional S3 URLs while DynamoDB/others still on boto3).
    let s3_boto_present = result.contains("client('s3'")
        || result.contains("client(\"s3\"")
        || result.contains("resource('s3'")
        || result.contains("resource(\"s3\"");
    if !s3_boto_present {
        result = result.replace("s3://", "gs://");
    }

    // === Lambda / Azure Functions → Cloud Functions fixup ===
    // Rewrite `def xxx(event, context):` → `@functions_framework.http\ndef xxx(request):`
    // and add `import functions_framework`. Done in fixup (not pattern) so inner SDK
    // patterns (DynamoDB, S3, etc.) can transform the function body independently.
    result = fix_lambda_to_cloud_functions(&result);

    // === Azure Functions import cleanup ===
    // Replace `import azure.functions as func` with `import functions_framework`
    // and rewrite decorated handlers.
    result = fix_azure_functions_to_cloud_functions(&result);

    // === Unresolved binding cleanup ===
    // Replace /* unresolved: ... */ with TODO comments
    result = fix_unresolved_bindings(&result);

    // === Add google.cloud.exceptions import if exception fixups were applied ===
    if result.contains("google.cloud.exceptions.")
        && !result.contains("from google.cloud import exceptions")
        && !result.contains("import google.cloud.exceptions")
    {
        // Find the last import line and add after it
        let import_line = "from google.cloud import exceptions\n";
        if let Some(pos) = find_last_import_position(&result) {
            result.insert_str(pos, import_line);
        }
    }

    result
}

/// Fix exception references from AWS client-style to GCP-style.
///
/// Handles both specific client-variable patterns (e.g. `s3.exceptions.ClientError`)
/// and generic `ClientError` references in except clauses.
fn fix_exception_references(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result_lines: Vec<String> = Vec::with_capacity(lines.len());

    // Bare `except ClientError` is correct for boto3/botocore. Only rewrite when
    // the file no longer uses boto3 (fully or primarily migrated to GCP SDK).
    let still_uses_boto3 = source.contains("import boto3")
        || source.contains("from boto3")
        || source.contains("botocore.exceptions");

    for line in &lines {
        let mut fixed = line.to_string();

        // Specific client-variable patterns: e.g. s3.exceptions.ClientError,
        // dynamodb.exceptions.*, sqs.exceptions.*, sns.exceptions.*
        // These are the most specific and safe to replace unconditionally.
        fixed = replace_client_exception_pattern(&fixed, "s3", "NotFound");
        fixed = replace_client_exception_pattern(&fixed, "dynamodb", "GoogleCloudError");
        fixed = replace_client_exception_pattern(&fixed, "sqs", "GoogleCloudError");
        fixed = replace_client_exception_pattern(&fixed, "sns", "GoogleCloudError");
        fixed = replace_client_exception_pattern(&fixed, "kinesis", "GoogleCloudError");
        fixed = replace_client_exception_pattern(&fixed, "lambda_client", "GoogleCloudError");
        // Do not use a generic "client.exceptions." match: it hits substrings like
        // dynamodb_client.exceptions and breaks valid boto3 error handling.

        // Generic ClientError in except clauses — only after boto3 is gone
        let trimmed = fixed.trim();
        if trimmed.starts_with("except") && fixed.contains("ClientError") && !still_uses_boto3 {
            fixed = fixed.replace("ClientError", "google.cloud.exceptions.GoogleCloudError");
        }

        result_lines.push(fixed);
    }

    result_lines.join("\n")
}

/// Replace `{client}.exceptions.{anything}` with `google.cloud.exceptions.{gcp_exception}`.
fn replace_client_exception_pattern(line: &str, client_name: &str, gcp_exception: &str) -> String {
    let pattern = format!("{client_name}.exceptions.");
    if let Some(start) = line.find(&pattern) {
        // Find the end of the exception class name (next non-identifier character)
        let after_pattern = start + pattern.len();
        let exception_end = line[after_pattern..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| after_pattern + i)
            .unwrap_or(line.len());
        let full_match = &line[start..exception_end];
        line.replace(
            full_match,
            &format!("google.cloud.exceptions.{gcp_exception}"),
        )
    } else {
        line.to_string()
    }
}

/// Fix the download_as_bytes + response['Body'].read() pattern.
///
/// When `download_as_bytes()` is assigned to a variable and the next line
/// accesses `var['Body'].read()`, we merge the two lines so the download
/// result (which IS bytes) flows directly into `.decode()` or similar.
fn fix_download_as_bytes_pattern(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result_lines: Vec<String> = Vec::with_capacity(lines.len());
    let mut skip_next = false;

    for i in 0..lines.len() {
        if skip_next {
            skip_next = false;
            continue;
        }

        let line = lines[i];
        let trimmed = line.trim();

        // Check if this line assigns download_as_bytes() to a variable
        if trimmed.contains("download_as_bytes()") && trimmed.contains('=') {
            // Extract the variable name (LHS of assignment)
            if let Some(eq_pos) = trimmed.find('=') {
                let var_name = trimmed[..eq_pos].trim();
                let rhs = trimmed[eq_pos + 1..].trim();
                let indent = &line[..line.len() - line.trim_start().len()];

                // Check if next line accesses var_name['Body'].read()
                if i + 1 < lines.len() {
                    let next = lines[i + 1].trim();
                    let body_read = format!("{}['Body'].read()", var_name);

                    if next.contains(&body_read) {
                        // Merge: replace var['Body'].read() with the download call directly
                        let merged = next.replace(&body_read, rhs);
                        result_lines.push(format!("{}{}", indent, merged.trim()));
                        skip_next = true;
                        continue;
                    }
                }
            }
        }

        result_lines.push(line.to_string());
    }

    result_lines.join("\n")
}

/// Fix the list_blobs + response.get('Contents', []) pattern.
///
/// When `list_blobs()` is assigned to a variable, downstream code that
/// accesses `.get('Contents', [])` and iterates with `obj['Key']` needs
/// to be updated: the variable becomes a list of Blob objects, so
/// `obj['Key']` becomes `blob.name` and the `.get('Contents', [])` wrapper
/// is removed.
fn fix_list_blobs_pattern(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let mut result_lines: Vec<String> = Vec::with_capacity(lines.len());
    let mut var_renames: Vec<(String, String)> = Vec::new();

    for line in &lines {
        let trimmed = line.trim();

        // Check if this line assigns list_blobs() result to a variable
        if trimmed.contains("list_blobs(") && trimmed.contains('=') {
            if let Some(eq_pos) = trimmed.find('=') {
                let var_name = trimmed[..eq_pos].trim().to_string();
                let new_var = "blobs".to_string();

                // Rename the variable to 'blobs' for clarity
                let new_line = line.replacen(&var_name, &new_var, 1);
                result_lines.push(new_line);
                var_renames.push((var_name, new_var));
                continue;
            }
        }

        let mut fixed_line = line.to_string();

        for (old_var, new_var) in &var_renames {
            // Fix: response.get('Contents', []) -> blobs
            let get_contents_single = format!("{}.get('Contents', [])", old_var);
            let get_contents_double = format!("{}.get(\"Contents\", [])", old_var);
            fixed_line = fixed_line.replace(&get_contents_single, new_var);
            fixed_line = fixed_line.replace(&get_contents_double, new_var);

            // Fix: obj['Key'] -> blob.name (in list comprehensions iterating over blobs)
            if fixed_line.contains(new_var) && fixed_line.contains("for") {
                fixed_line = fixed_line.replace("obj['Key']", "blob.name");
                fixed_line = fixed_line.replace("obj[\"Key\"]", "blob.name");
                fixed_line = fixed_line.replace("for obj in", "for blob in");
            }
        }

        result_lines.push(fixed_line);
    }

    result_lines.join("\n")
}

/// Rewrite Lambda handler signatures to Cloud Functions format.
///
/// Detects `def xxx(event, context):` (where the second param is literally
/// `context`) and rewrites to `@functions_framework.http\ndef xxx(request):`.
/// Also adds `import functions_framework` if not already present.
///
/// Only fires when boto3 was present (i.e. other patterns ran), indicated by
/// a google.cloud import or functions_framework already being imported.
fn fix_lambda_to_cloud_functions(source: &str) -> String {
    // Only run when the file had cloud transforms applied (google.cloud present).
    // Avoids false positives on random (event, context) functions.
    if !source.contains("google.cloud") {
        return source.to_string();
    }

    let mut lines: Vec<String> = source.lines().map(String::from).collect();
    let mut changed = false;
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim_start().to_string();
        // Match: def func_name(xxx, context):
        if trimmed.starts_with("def ") && trimmed.ends_with(':') {
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = trimmed.rfind(')') {
                    let params = trimmed[paren_start + 1..paren_end].trim().to_string();
                    // Check for exactly two params where second is "context"
                    let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 2
                        && parts[1] == "context"
                        && parts[0] != "self"
                        && parts[0] != "cls"
                    {
                        let indent_len = lines[i].len() - trimmed.len();
                        let indent = lines[i][..indent_len].to_string();
                        let func_name = trimmed[4..paren_start].trim().to_string();

                        // Check not already decorated
                        let already_decorated = i > 0
                            && lines[..i]
                                .iter()
                                .rev()
                                .find(|l| !l.trim().is_empty())
                                .map(|l| l.trim().starts_with("@functions_framework"))
                                .unwrap_or(false);

                        if !already_decorated {
                            lines.insert(i, format!("{indent}@functions_framework.http"));
                            i += 1; // skip past decorator
                        }

                        // Rewrite function signature
                        lines[i] = format!("{indent}def {func_name}(request):");
                        changed = true;
                    }
                }
            }
        }
        i += 1;
    }

    if !changed {
        return source.to_string();
    }

    let mut result = lines.join("\n");

    // Add import if not present
    if !result.contains("import functions_framework") {
        if let Some(pos) = find_last_import_position(&result) {
            result.insert_str(pos, "import functions_framework\n");
        }
    }

    result
}

/// Rewrite Azure Functions patterns to Cloud Functions.
///
/// Handles `import azure.functions as func`, `func.FunctionApp()`,
/// and decorated handlers with `@app.xxx_trigger(...)`.
fn fix_azure_functions_to_cloud_functions(source: &str) -> String {
    if !source.contains("azure.functions") && !source.contains("func.FunctionApp") {
        return source.to_string();
    }

    let mut result = source.to_string();

    // Replace import
    result = result.replace(
        "import azure.functions as func",
        "import functions_framework",
    );

    // Remove func.FunctionApp() line
    let lines: Vec<&str> = result.lines().collect();
    let filtered: Vec<&str> = lines
        .into_iter()
        .filter(|l| !l.contains("func.FunctionApp()"))
        .collect();
    result = filtered.join("\n");

    // Replace Azure trigger decorators with @functions_framework.http
    let mut lines: Vec<String> = result.lines().map(String::from).collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim();
        // Remove @app.function_name(...) and @app.xxx_trigger(...) decorators
        if trimmed.starts_with("@app.function_name(")
            || trimmed.starts_with("@app.service_bus_queue_trigger(")
            || trimmed.starts_with("@app.route(")
            || trimmed.starts_with("@app.queue_trigger(")
            || trimmed.starts_with("@app.blob_trigger(")
            || trimmed.starts_with("@app.timer_trigger(")
            || trimmed.starts_with("@app.event_hub_trigger(")
        {
            lines.remove(i);
            continue;
        }
        i += 1;
    }
    result = lines.join("\n");

    // Rewrite typed Azure Functions params: def xxx(msg: func.ServiceBusMessage):
    // → @functions_framework.http\ndef xxx(request):
    let mut lines: Vec<String> = result.lines().map(String::from).collect();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start().to_string();
        if trimmed.starts_with("def ") && trimmed.ends_with(':') {
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = trimmed.rfind(')') {
                    let params = trimmed[paren_start + 1..paren_end].trim().to_string();
                    // Match: single param with func.XxxMessage type annotation
                    if params.contains("func.") || params.contains("azure.functions") {
                        let indent_len = lines[i].len() - trimmed.len();
                        let indent = lines[i][..indent_len].to_string();
                        let func_name = trimmed[4..paren_start].trim().to_string();

                        let already_decorated = i > 0
                            && lines[..i]
                                .iter()
                                .rev()
                                .find(|l| !l.trim().is_empty())
                                .map(|l| l.trim().starts_with("@functions_framework"))
                                .unwrap_or(false);

                        if !already_decorated {
                            lines.insert(i, format!("{indent}@functions_framework.http"));
                            i += 1;
                        }

                        lines[i] = format!("{indent}def {func_name}(request):");
                    }
                }
            }
        }
        i += 1;
    }

    lines.join("\n")
}

/// Replace `/* unresolved: ... */` markers with `# TODO: resolve` comments.
fn fix_unresolved_bindings(source: &str) -> String {
    let mut result = source.to_string();
    while let Some(start) = result.find("/* unresolved:") {
        if let Some(end_offset) = result[start..].find("*/") {
            let end = start + end_offset + 2;
            let unresolved = result[start..end].to_string();
            let field = unresolved
                .strip_prefix("/* unresolved: ")
                .unwrap_or(&unresolved)
                .strip_suffix(" */")
                .unwrap_or(&unresolved);
            result = result.replacen(&unresolved, &format!("{field}  # TODO: resolve"), 1);
        } else {
            break;
        }
    }
    result
}

fn apply_typescript_fixups(source: &str) -> String {
    let mut result = source.to_string();

    // Fix S3 URI scheme
    result = result.replace("s3://", "gs://");

    // Fix AWS exception references
    result = result.replace("AWSError", "Error");

    result
}

/// Find the byte position after the last import line.
fn find_last_import_position(source: &str) -> Option<usize> {
    let mut last_import_end = None;
    let mut pos = 0;
    for line in source.lines() {
        let next_pos = pos + line.len() + 1; // +1 for newline
        let trimmed = line.trim();
        if trimmed.starts_with("import ") || trimmed.starts_with("from ") {
            last_import_end = Some(next_pos.min(source.len()));
        }
        pos = next_pos;
    }
    last_import_end
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_download_as_bytes() {
        let source = r#"    response = storage.Client().bucket(B).blob(K).download_as_bytes()
    content = response['Body'].read().decode('utf-8')
    return json.loads(content)"#;

        let fixed = fix_download_as_bytes_pattern(source);
        assert!(
            !fixed.contains("response['Body'].read()"),
            "Should remove response['Body'].read(), got:\n{fixed}"
        );
        assert!(
            fixed.contains("download_as_bytes().decode('utf-8')"),
            "Should merge download_as_bytes() with .decode(), got:\n{fixed}"
        );
        assert!(
            fixed.contains("return json.loads(content)"),
            "Should preserve subsequent lines, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_download_as_bytes_preserves_indentation() {
        let source = "        response = bucket.blob(K).download_as_bytes()\n        content = response['Body'].read().decode('utf-8')";
        let fixed = fix_download_as_bytes_pattern(source);
        assert!(
            fixed.starts_with("        content"),
            "Should preserve leading whitespace, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_list_blobs() {
        let source = r#"    response = list(storage.Client().bucket(B).list_blobs(prefix=p))
    return [obj['Key'] for obj in response.get('Contents', [])]"#;

        let fixed = fix_list_blobs_pattern(source);
        assert!(
            !fixed.contains("response.get('Contents', [])"),
            "Should remove .get('Contents', []), got:\n{fixed}"
        );
        assert!(
            fixed.contains("blob.name"),
            "Should replace obj['Key'] with blob.name, got:\n{fixed}"
        );
        assert!(
            fixed.contains("for blob in"),
            "Should replace 'for obj in' with 'for blob in', got:\n{fixed}"
        );
    }

    #[test]
    fn test_s3_uri_fixup() {
        let source = "return f\"s3://{BUCKET_NAME}/documents/{key}.json\"";
        let fixed = apply_python_fixups(source);
        assert!(
            fixed.contains("gs://"),
            "Should replace s3:// with gs://, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("s3://"),
            "Should not contain s3:// after fixup, got:\n{fixed}"
        );
    }

    #[test]
    fn test_exception_fixup() {
        let source = "    except s3.exceptions.ClientError:\n        return False";
        let fixed = apply_python_fixups(source);
        assert!(
            fixed.contains("google.cloud.exceptions.NotFound"),
            "Should replace s3.exceptions.ClientError with google.cloud.exceptions.NotFound, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("s3.exceptions.ClientError"),
            "Should not contain original exception, got:\n{fixed}"
        );
    }

    #[test]
    fn test_exception_fixup_adds_import() {
        let source = "from google.cloud import storage\n\n    except s3.exceptions.ClientError:\n        return False";
        let fixed = apply_python_fixups(source);
        assert!(
            fixed.contains("from google.cloud import exceptions"),
            "Should add exceptions import, got:\n{fixed}"
        );
    }

    #[test]
    fn test_exception_fixup_does_not_duplicate_import() {
        let source = "from google.cloud import exceptions\n\n    except s3.exceptions.ClientError:\n        return False";
        let fixed = apply_python_fixups(source);
        let count = fixed.matches("from google.cloud import exceptions").count();
        assert_eq!(
            count, 1,
            "Should not duplicate the exceptions import, got:\n{fixed}"
        );
    }

    #[test]
    fn test_unresolved_binding_cleanup() {
        let source = "storage.Client().bucket(/* unresolved: args.CopySource.Bucket */).blob(key)";
        let fixed = apply_python_fixups(source);
        assert!(
            !fixed.contains("/* unresolved"),
            "Should remove /* unresolved markers, got:\n{fixed}"
        );
        assert!(
            fixed.contains("# TODO: resolve"),
            "Should add TODO comment, got:\n{fixed}"
        );
    }

    #[test]
    fn test_typescript_s3_uri_fixup() {
        let source = "const url = `s3://${bucket}/key`";
        let fixed = apply_typescript_fixups(source);
        assert!(fixed.contains("gs://"));
        assert!(!fixed.contains("s3://"));
    }

    #[test]
    fn test_apply_fixups_dispatches_python() {
        let source = "x = \"s3://bucket/key\"";
        let fixed = apply_fixups(source, Language::Python);
        assert!(fixed.contains("gs://"));
    }

    #[test]
    fn test_apply_fixups_noop_for_other_languages() {
        let source = "x = \"s3://bucket/key\"";
        let fixed = apply_fixups(source, Language::Java);
        assert_eq!(fixed, source);
    }

    #[test]
    fn test_s3_uri_not_rewritten_when_s3_boto_client_present() {
        let source = r#"import boto3
c = boto3.client('s3')
u = "s3://bucket/key"
"#;
        let fixed = apply_python_fixups(source);
        assert!(
            fixed.contains("s3://"),
            "Should keep s3:// while S3 client exists:\n{fixed}"
        );
    }

    #[test]
    fn test_s3_uri_rewritten_when_only_non_s3_boto_client() {
        let source = r#"import boto3
c = boto3.client('dynamodb')
u = "s3://bucket/key"
"#;
        let fixed = apply_python_fixups(source);
        assert!(
            fixed.contains("gs://"),
            "Should rewrite s3:// when no S3 boto client:\n{fixed}"
        );
    }

    #[test]
    fn test_preserves_botocore_client_error_when_boto3_present() {
        let source = r#"import boto3
from botocore.exceptions import ClientError

def create_role():
    try:
        boto3.client("iam").create_role(RoleName="x", AssumeRolePolicyDocument="{}")
    except ClientError as e:
        print(e)
"#;
        let fixed = apply_python_fixups(source);
        assert!(
            fixed.contains("except ClientError as e"),
            "Should keep botocore ClientError when boto3 is in use, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("google.cloud.exceptions.GoogleCloudError"),
            "Should not inject GCP exception for boto3 code, got:\n{fixed}"
        );
    }

    #[test]
    fn test_full_python_fixup_integration() {
        // Simulate what the pipeline would produce after pattern transforms
        // but before fixups.
        let source = r#"from google.cloud import storage
import json
from datetime import datetime, timedelta

s3 = storage.Client()
BUCKET_NAME = 'my-data-pipeline'

def download_document(key: str) -> dict:
    response = storage.Client().bucket(BUCKET_NAME).blob(key).download_as_bytes()
    content = response['Body'].read().decode('utf-8')
    return json.loads(content)

def list_documents(prefix: str, max_keys: int = 100) -> list:
    response = list(storage.Client().bucket(BUCKET_NAME).list_blobs(prefix=prefix))
    return [obj['Key'] for obj in response.get('Contents', [])]

def check_document_exists(key: str) -> bool:
    try:
        storage.Client().bucket(BUCKET_NAME).blob(key).exists()
        return True
    except s3.exceptions.ClientError:
        return False

def upload_json_document(key: str, data: dict) -> str:
    body = json.dumps(data, default=str)
    storage.Client().bucket(BUCKET_NAME).blob(f"documents/{key}.json").upload_from_string(body, content_type='application/json')
    return f"s3://{BUCKET_NAME}/documents/{key}.json"
"#;

        let fixed = apply_python_fixups(source);

        // download_as_bytes pattern should be merged
        assert!(
            !fixed.contains("response['Body'].read()"),
            "download_as_bytes pattern not fixed:\n{fixed}"
        );

        // list_blobs pattern should be fixed
        assert!(
            !fixed.contains("response.get('Contents', [])"),
            "list_blobs pattern not fixed:\n{fixed}"
        );
        assert!(
            fixed.contains("blob.name"),
            "list_blobs obj['Key'] not replaced:\n{fixed}"
        );

        // Exception should be fixed
        assert!(
            !fixed.contains("s3.exceptions.ClientError"),
            "exception not fixed:\n{fixed}"
        );
        assert!(
            fixed.contains("google.cloud.exceptions.NotFound"),
            "exception not replaced with GCP equivalent:\n{fixed}"
        );

        // URI scheme should be fixed
        assert!(
            fixed.contains("gs://"),
            "s3:// not replaced with gs://:\n{fixed}"
        );
        assert!(!fixed.contains("s3://"), "s3:// still present:\n{fixed}");

        // exceptions import should be added
        assert!(
            fixed.contains("from google.cloud import exceptions"),
            "exceptions import not added:\n{fixed}"
        );
    }

    // ---------------------------------------------------------------------------
    // DynamoDB → standard JSON marshaling tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_dynamodb_s_to_standard() {
        let v = serde_json::json!({"S": "hello"});
        let out = dynamodb_item_to_standard_json(&v).unwrap();
        assert_eq!(out, serde_json::json!("hello"));
    }

    #[test]
    fn test_dynamodb_n_to_standard() {
        let v = serde_json::json!({"N": "42"});
        let out = dynamodb_item_to_standard_json(&v).unwrap();
        assert_eq!(out, serde_json::json!(42));
    }

    #[test]
    fn test_dynamodb_m_to_standard() {
        let v = serde_json::json!({"M": {"id": {"S": "1"}, "count": {"N": "10"}}});
        let out = dynamodb_item_to_standard_json(&v).unwrap();
        assert_eq!(out, serde_json::json!({"id": "1", "count": 10}));
    }

    #[test]
    fn test_dynamodb_item_map_to_standard() {
        let item = r#"{"id": {"S": "user-1"}, "name": {"S": "Alice"}, "score": {"N": "99"}}"#;
        let out = dynamodb_item_json_string_to_standard(item).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["id"], "user-1");
        assert_eq!(parsed["name"], "Alice");
        assert_eq!(parsed["score"], 99);
    }

    #[test]
    fn test_dynamodb_plain_json_passthrough() {
        let item = r#"{"id": "1", "name": "Bob"}"#;
        let out = dynamodb_item_json_string_to_standard(item).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(parsed["id"], "1");
        assert_eq!(parsed["name"], "Bob");
    }
}
