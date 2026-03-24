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

    // === Leftover Azure SDK import removal ===
    // After pattern transforms replace Azure calls with GCP equivalents, leftover
    // `from azure.xxx import ...` lines may remain. Remove them.
    result = remove_leftover_azure_imports(&result);

    // === DynamoDB → Firestore semantic fixups ===
    // After pattern transforms replace boto3.resource('dynamodb') → firestore.Client(),
    // the surrounding DynamoDB idioms need rewriting to Firestore equivalents.
    if result.contains("firestore") {
        result = fix_dynamodb_to_firestore_semantics(&result);
    }

    // === Azure client init leftover cleanup ===
    // After Azure method-call patterns fire (upload_blob → upload_from_string, etc.),
    // the client initialization lines (BlobServiceClient.from_connection_string, etc.)
    // remain as Azure code. This fixup removes or replaces them when google.cloud
    // imports are present, preventing broken "cloud cocktails."
    result = fix_azure_client_init_leftovers(&result);

    // === AWS response access pattern fixups ===
    // After patterns transform API calls (e.g. Secrets Manager, KMS, Pub/Sub, Compute),
    // the surrounding code still accesses the response using AWS dict-key patterns
    // that don't exist in the GCP SDK. Rewrite these to GCP equivalents.
    result = fix_aws_response_patterns(&result);

    // === KMS variable shadowing fixup ===
    // `kms = kms.KeyManagementServiceClient()` shadows the module import.
    // Rename the variable to `kms_client` throughout.
    result = fix_kms_variable_shadowing(&result);

    // === AWS → GCP client variable renaming ===
    // After patterns assign GCP clients to AWS-named variables
    // (e.g. `sns = pubsub_v1.PublisherClient()`), rename to idiomatic GCP names.
    result = fix_aws_variable_names(&result);

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
    let mut event_param: Option<String> = None;
    let mut i = 0;

    while i < lines.len() {
        let trimmed = lines[i].trim_start().to_string();
        // Match: def func_name(xxx, context):
        if trimmed.starts_with("def ") && trimmed.ends_with(':') {
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = trimmed.rfind(')') {
                    let params = trimmed[paren_start + 1..paren_end].trim().to_string();
                    let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
                    if parts.len() == 2
                        && parts[1] == "context"
                        && parts[0] != "self"
                        && parts[0] != "cls"
                    {
                        let indent_len = lines[i].len() - trimmed.len();
                        let indent = lines[i][..indent_len].to_string();
                        let func_name = trimmed[4..paren_start].trim().to_string();
                        event_param = Some(parts[0].to_string());

                        // Add decorator
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

                        // Rewrite signature
                        lines[i] = format!("{indent}def {func_name}(request):");
                        i += 1;

                        // Detect body indent (next non-empty line)
                        let body_indent = if i < lines.len() {
                            let next = &lines[i];
                            let next_trimmed = next.trim_start();
                            next[..next.len() - next_trimmed.len()].to_string()
                        } else {
                            format!("{indent}    ")
                        };

                        // Insert request_json extraction as first body line
                        lines.insert(
                            i,
                            format!("{body_indent}request_json = request.get_json(silent=True)"),
                        );
                        i += 1;

                        changed = true;
                        continue; // skip the normal i += 1
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

    // Replace event parameter references with request_json
    if let Some(ref evt) = event_param {
        // event['x'] → request_json['x']
        result = result.replace(&format!("{evt}['"), "request_json['");
        result = result.replace(&format!("{evt}[\""), "request_json[\"");
        // json.dumps(event) → json.dumps(request_json)
        result = result.replace(&format!("({evt})"), "(request_json)");
        result = result.replace(&format!(" {evt})"), " request_json)");
    }

    // Fix Lambda return format: {'statusCode': 200, 'body': json.dumps(X)}
    // → (json.dumps(X), 200, {'Content-Type': 'application/json'})
    if result.contains("'statusCode'") && result.contains("'body'") {
        // Simple case: return {'statusCode': 200, 'body': json.dumps({'ok': True})}
        // We'll use a line-by-line approach
        let lines: Vec<String> = result.lines().map(String::from).collect();
        let mut new_lines = Vec::new();
        for line in &lines {
            let trimmed = line.trim();
            if trimmed.starts_with("return {")
                && trimmed.contains("'statusCode'")
                && trimmed.contains("'body'")
            {
                let indent = &line[..line.len() - trimmed.len()];
                // Extract the body value between 'body': and the closing }
                if let Some(body_start) = trimmed.find("'body':") {
                    let after_body = &trimmed[body_start + 8..]; // skip "'body': "
                    let body_val = after_body
                        .trim()
                        .trim_end_matches('}')
                        .trim()
                        .trim_end_matches(',')
                        .trim();
                    new_lines.push(format!(
                        "{indent}return ({body_val}, 200, {{'Content-Type': 'application/json'}})"
                    ));
                    continue;
                }
            }
            new_lines.push(line.clone());
        }
        result = new_lines.join("\n");
    }

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
    let mut old_param_names: Vec<String> = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let trimmed = lines[i].trim_start().to_string();
        if trimmed.starts_with("def ") && trimmed.ends_with(':') {
            if let Some(paren_start) = trimmed.find('(') {
                if let Some(paren_end) = trimmed.rfind(')') {
                    let params = trimmed[paren_start + 1..paren_end].trim().to_string();
                    // Match: single param with func.XxxMessage type annotation
                    if params.contains("func.") || params.contains("azure.functions") {
                        // Extract old parameter name (before the colon/type annotation)
                        let old_param = params.split(':').next().unwrap_or("").trim().to_string();
                        if !old_param.is_empty() && old_param != "request" {
                            old_param_names.push(old_param);
                        }

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

    let mut result = lines.join("\n");

    // Replace old Azure parameter body references with Cloud Functions equivalents
    for old_param in &old_param_names {
        // msg.get_body().decode() → request.get_json(silent=True)
        let get_body_decode = format!("{old_param}.get_body().decode()");
        result = result.replace(&get_body_decode, "request.get_json(silent=True)");

        // msg.get_body() → request.data
        let get_body = format!("{old_param}.get_body()");
        result = result.replace(&get_body, "request.data");
    }

    result
}

/// Remove leftover `from azure.xxx import ...` lines after pattern transforms
/// have replaced the Azure API calls with GCP equivalents.
fn remove_leftover_azure_imports(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    let filtered: Vec<&str> = lines
        .into_iter()
        .filter(|l| {
            let trimmed = l.trim();
            // Remove `from azure.eventhub import ...`
            // Remove `from azure.servicebus import ...`
            if trimmed.starts_with("from azure.eventhub ")
                || trimmed.starts_with("from azure.servicebus ")
            {
                return false;
            }
            true
        })
        .collect();
    filtered.join("\n")
}

/// Rewrite DynamoDB idioms to Firestore equivalents after pattern transforms.
///
/// Handles:
/// - `.Table('name')` → `.collection('name')`
/// - `table.set({...})` → `db.collection('x').document(DOC_ID).set({...})`
///   (when the variable was bound to a `.collection()` call)
/// - `.get(Key={...})` → `.document(key).get()`
/// - `.get('Item')` / `['Items']` → Firestore result access
/// - Moves client init outside functions for warm-start performance
fn fix_dynamodb_to_firestore_semantics(source: &str) -> String {
    let mut result = source.to_string();

    // Rename DynamoDB-style variable names
    // dynamodb = firestore.Client() → db = firestore.Client()
    if result.contains("dynamodb = firestore.Client()") {
        result = result.replace("dynamodb = firestore.Client()", "db = firestore.Client()");
        result = result.replace("dynamodb.", "db.");
        // Also fix: table = db.collection(...) → just use db.collection() directly
        // but keep the variable for now as it's more readable
    }

    // lambda_handler → handle_request (when functions_framework is present)
    if result.contains("functions_framework") && result.contains("def lambda_handler(") {
        result = result.replace("def lambda_handler(", "def handle_request(");
    }

    // .Table('name') → .collection('name')
    result = result.replace(".Table('", ".collection('");
    result = result.replace(".Table(\"", ".collection(\"");

    // response.get('Item') → response.to_dict() (Firestore document snapshot)
    result = result.replace(".get('Item')", ".to_dict()");
    result = result.replace(".get(\"Item\")", ".to_dict()");

    // response['Items'] → [doc.to_dict() for doc in response]
    result = result.replace("['Items']", "");
    result = result.replace("[\"Items\"]", "");

    // .get(Key={'id': x}) → .document(str(x)).get()  — simple single-key case
    // This is a heuristic for the common pattern
    let lines: Vec<String> = result.lines().map(String::from).collect();
    let mut new_lines = Vec::new();
    for line in &lines {
        let trimmed = line.trim();
        // Pattern: xxx.get(Key={'field': value})
        if trimmed.contains(".get(Key={") && trimmed.contains("})") {
            // Extract the value: everything between ': ' and '}'
            if let Some(colon_pos) = trimmed.find("': ") {
                let after_colon = &trimmed[colon_pos + 3..];
                if let Some(brace_pos) = after_colon.find('}') {
                    let value = &after_colon[..brace_pos];
                    let indent = &line[..line.len() - trimmed.len()];
                    // Find the object.get( part
                    if let Some(get_pos) = trimmed.find(".get(Key=") {
                        let obj = &trimmed[..get_pos];
                        // Check for assignment
                        let rest_after_close = &trimmed[trimmed.find("})").unwrap() + 2..];
                        new_lines.push(format!(
                            "{indent}{obj}.document(str({value})).get(){rest_after_close}"
                        ));
                        continue;
                    }
                }
            }
        }
        new_lines.push(line.clone());
    }
    result = new_lines.join("\n");

    // .set({...}) on a collection variable → .document(DOC_ID).set({...})
    // When a variable was assigned to xxx.collection('name'), calls like
    // table.set({...}) need to become table.document(DOC_ID).set({...})
    //
    // Heuristic: if line contains var.set({...}) and we know var was bound
    // to a .collection() result, insert .document(DOC_ID) before .set()
    // For simplicity, detect: xxx.set({ and replace with xxx.document(DOC_ID).set({
    // only when firestore.Client() is in the file.
    let lines: Vec<String> = result.lines().map(String::from).collect();
    let mut new_lines = Vec::new();

    // Find which variables are collection references
    let mut collection_vars: Vec<String> = Vec::new();
    for line in &lines {
        let trimmed = line.trim();
        // pattern: var = xxx.collection('name')
        if trimmed.contains(".collection(") {
            if let Some(eq_pos) = trimmed.find(" = ") {
                let var = trimmed[..eq_pos].trim().to_string();
                collection_vars.push(var);
            }
        }
    }

    for line in &lines {
        let trimmed = line.trim();
        let mut replaced = false;
        for var in &collection_vars {
            let set_pattern = format!("{var}.set(");
            if trimmed.contains(&set_pattern) {
                let indent = &line[..line.len() - trimmed.len()];
                // Try to extract document ID from the dict: .set({'id': X, ...})
                // Look for 'id': VALUE pattern in the set() args
                let doc_id = extract_doc_id_from_set_call(trimmed);
                let new_line =
                    trimmed.replace(&set_pattern, &format!("{var}.document(str({doc_id})).set("));
                new_lines.push(format!("{indent}{new_line}"));
                replaced = true;
                break;
            }
        }
        if !replaced {
            new_lines.push(line.clone());
        }
    }

    new_lines.join("\n")
}

/// Extract the document ID expression from a .set({...}) call.
/// Looks for 'id': VALUE in the dict literal.
fn extract_doc_id_from_set_call(line: &str) -> String {
    // Try to find 'id': <value> pattern
    for prefix in ["'id': ", "\"id\": "] {
        if let Some(pos) = line.find(prefix) {
            let after = &line[pos + prefix.len()..];
            // Find the end of the value expression (comma or closing brace)
            let mut depth = 0;
            let mut end = 0;
            for (i, ch) in after.char_indices() {
                match ch {
                    '(' | '[' | '{' => depth += 1,
                    ')' | ']' | '}' => {
                        if depth == 0 {
                            end = i;
                            break;
                        }
                        depth -= 1;
                    }
                    ',' if depth == 0 => {
                        end = i;
                        break;
                    }
                    _ => {}
                }
            }
            if end > 0 {
                return after[..end].trim().to_string();
            }
        }
    }
    "DOC_ID".to_string()
}

/// Remove or replace Azure client initialization leftovers after pattern transforms.
///
/// When Azure method-call patterns fire (e.g. `upload_blob` → `upload_from_string`),
/// the client init lines (`BlobServiceClient.from_connection_string(...)`, etc.) stay
/// behind because only the *method calls* are matched by tree-sitter patterns.
/// This fixup cleans them up, but only when google.cloud imports confirm that
/// pattern transforms actually ran (conservative guard).
fn fix_azure_client_init_leftovers(source: &str) -> String {
    // Only act when GCP imports are present (patterns fired).
    let has_gcp =
        source.contains("from google.cloud import") || source.contains("from google.cloud.");
    if !has_gcp {
        return source.to_string();
    }

    let lines: Vec<&str> = source.lines().collect();
    let mut result_lines: Vec<String> = Vec::with_capacity(lines.len());

    for line in &lines {
        let trimmed = line.trim();

        // ── Remove Azure import lines ──────────────────────────────────
        // Catches `from azure.storage.blob import ...`, `from azure.identity import ...`,
        // `import azure.cosmos`, etc. Excludes azure.functions (separate fixup).
        if (trimmed.starts_with("from azure.") || trimmed.starts_with("import azure."))
            && !trimmed.contains("azure.functions")
        {
            continue;
        }

        // ── BlobServiceClient.from_connection_string(...) ──────────────
        // Patterns: assignment or bare call
        if trimmed.contains("BlobServiceClient.from_connection_string(")
            || trimmed.contains("BlobServiceClient(")
        {
            // Replace with storage.Client() if assigned to a variable
            if let Some(eq_pos) = trimmed.find('=') {
                let lhs = trimmed[..eq_pos].trim();
                if !lhs.contains('(') {
                    // It's an assignment: var = BlobServiceClient...
                    let indent = &line[..line.len() - trimmed.len()];
                    result_lines.push(format!("{indent}{lhs} = storage.Client()"));
                    continue;
                }
            }
            // Bare call or complex expression — just remove it
            continue;
        }

        // ── container = xxx.get_container_client("name") ───────────────
        if trimmed.contains(".get_container_client(") {
            if let Some(eq_pos) = trimmed.find('=') {
                let lhs = trimmed[..eq_pos].trim();
                if !lhs.contains('(') {
                    let indent = &line[..line.len() - trimmed.len()];
                    // Extract the container/bucket name argument
                    if let Some(arg) = extract_first_call_arg(trimmed, "get_container_client") {
                        result_lines
                            .push(format!("{indent}{lhs} = storage.Client().bucket({arg})"));
                        continue;
                    }
                }
            }
            // Not an assignment — remove
            continue;
        }

        // ── blob = container.get_blob_client(name) ─────────────────────
        // GCS doesn't need an intermediate blob client; the blob is accessed
        // inline via bucket.blob(name) in the transformed method calls.
        if trimmed.contains(".get_blob_client(") {
            continue;
        }

        // ── ServiceBusClient.from_connection_string(...) ───────────────
        if trimmed.contains("ServiceBusClient.from_connection_string(")
            || trimmed.contains("ServiceBusClient(")
        {
            continue;
        }

        // ── xxx.get_queue_sender(...) / get_queue_receiver(...) ────────
        if trimmed.contains(".get_queue_sender(") || trimmed.contains(".get_queue_receiver(") {
            continue;
        }

        // ── SecretClient(vault_url=..., credential=...) ────────────────
        if trimmed.contains("SecretClient(") && trimmed.contains("vault_url") {
            if let Some(eq_pos) = trimmed.find('=') {
                let lhs = trimmed[..eq_pos].trim();
                if !lhs.contains('(') {
                    let indent = &line[..line.len() - trimmed.len()];
                    result_lines.push(format!(
                        "{indent}{lhs} = secretmanager.SecretManagerServiceClient()"
                    ));
                    continue;
                }
            }
            continue;
        }

        // ── MetricsQueryClient(DefaultAzureCredential()) ──────────────
        if trimmed.contains("MetricsQueryClient(") {
            if let Some(eq_pos) = trimmed.find('=') {
                let lhs = trimmed[..eq_pos].trim();
                if !lhs.contains('(') {
                    let indent = &line[..line.len() - trimmed.len()];
                    result_lines.push(format!(
                        "{indent}{lhs} = monitoring_v3.MetricServiceClient()"
                    ));
                    continue;
                }
            }
            continue;
        }

        // ── QueueServiceClient.from_connection_string(...) ─────────────
        if trimmed.contains("QueueServiceClient.from_connection_string(")
            || trimmed.contains("QueueServiceClient(")
        {
            if let Some(eq_pos) = trimmed.find('=') {
                let lhs = trimmed[..eq_pos].trim();
                if !lhs.contains('(') {
                    let indent = &line[..line.len() - trimmed.len()];
                    result_lines.push(format!("{indent}{lhs} = pubsub_v1.PublisherClient()"));
                    continue;
                }
            }
            continue;
        }

        // ── xxx.get_queue_client("name") ───────────────────────────────
        if trimmed.contains(".get_queue_client(") {
            continue;
        }

        // ── CosmosClient(url, credential=key) ──────────────────────────
        if trimmed.contains("CosmosClient(") {
            continue;
        }

        // ── client.get_database_client("app") ─────────────────────────
        if trimmed.contains(".get_database_client(") {
            continue;
        }

        // ── db.get_container_client("users") (Cosmos, not Blob) ────────
        // .get_container_client was already handled above for Blob;
        // if we reach here it means the blob handler didn't catch it
        // (e.g. no assignment). Already handled — this is a safety net.

        // ── EventHubProducerClient.from_connection_string(...) ─────────
        if trimmed.contains("EventHubProducerClient.from_connection_string(")
            || trimmed.contains("EventHubProducerClient(")
        {
            continue;
        }

        // ── DefaultAzureCredential() bare usage ────────────────────────
        // Sometimes appears as standalone: credential = DefaultAzureCredential()
        if trimmed.contains("DefaultAzureCredential()") {
            continue;
        }

        // ── Fix .value → .payload.data.decode("utf-8") ────────────────
        // Azure KeyVault returns secret.value; GCP Secret Manager returns
        // response.payload.data (bytes).
        if trimmed.contains(".value")
            && source.contains("secretmanager")
            && !trimmed.contains(".payload.data")
        {
            let fixed = line.replace(".value", ".payload.data.decode(\"utf-8\")");
            result_lines.push(fixed);
            continue;
        }

        result_lines.push(line.to_string());
    }

    result_lines.join("\n")
}

/// Extract the first argument from a method call: `obj.method(ARG, ...)` → `ARG`.
/// Returns the argument including any quotes.
fn extract_first_call_arg(line: &str, method_name: &str) -> Option<String> {
    let pattern = format!("{method_name}(");
    let start = line.find(&pattern)? + pattern.len();
    let rest = &line[start..];
    let mut depth = 0;
    let mut end = 0;
    for (i, ch) in rest.char_indices() {
        match ch {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth == 0 {
                    end = i;
                    break;
                }
                depth -= 1;
            }
            ',' if depth == 0 => {
                end = i;
                break;
            }
            _ => {}
        }
    }
    if end > 0 {
        Some(rest[..end].trim().to_string())
    } else {
        // Single argument with no comma — take everything up to closing paren
        let close = rest.find(')')?;
        Some(rest[..close].trim().to_string())
    }
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

/// Rewrite AWS SDK response access patterns to GCP equivalents.
///
/// After patterns transform API calls from AWS to GCP, the code still accesses
/// the response using AWS dict-key patterns (e.g. `resp['SecretString']`) that
/// don't exist in GCP responses. This function rewrites those patterns, but
/// only when the corresponding GCP import is present to avoid false positives.
fn fix_aws_response_patterns(source: &str) -> String {
    let mut result = source.to_string();

    // --- Secret Manager patterns ---
    // Only apply when secretmanager is imported
    let has_secretmanager = result.contains("secretmanager");

    if has_secretmanager {
        // resp['SecretString'] → resp.payload.data.decode("utf-8")
        result = result.replace("['SecretString']", ".payload.data.decode(\"utf-8\")");
        result = result.replace("[\"SecretString\"]", ".payload.data.decode(\"utf-8\")");

        // resp['SecretBinary'] → resp.payload.data
        result = result.replace("['SecretBinary']", ".payload.data");
        result = result.replace("[\"SecretBinary\"]", ".payload.data");

        // r['Parameter']['Value'] → r.payload.data.decode("utf-8")
        // (SSM GetParameter transformed to Secret Manager)
        result = result.replace("['Parameter']['Value']", ".payload.data.decode(\"utf-8\")");
        result = result.replace(
            "[\"Parameter\"][\"Value\"]",
            ".payload.data.decode(\"utf-8\")",
        );

        // .value on Secret Manager response → .payload.data.decode("utf-8")
        // Only rewrite `.value` when it follows a variable on a line that looks
        // like a Secret Manager access (heuristic: same line references a
        // response variable but NOT a dict/attribute that legitimately has `.value`).
        let lines: Vec<String> = result.lines().map(String::from).collect();
        let mut new_lines = Vec::with_capacity(lines.len());
        for line in &lines {
            let trimmed = line.trim();
            // Match: xxx.value where xxx is likely a secret response
            // Guard: don't rewrite .value in dict literals, class defs, or unrelated contexts
            if trimmed.contains(".value")
                && !trimmed.contains(".value.")
                && !trimmed.contains(".values")
                && !trimmed.starts_with("class ")
                && !trimmed.starts_with("def ")
                && !trimmed.contains("# ")
                && (trimmed.contains("secret")
                    || trimmed.contains("response")
                    || trimmed.contains("resp"))
            {
                // Replace .value with .payload.data.decode("utf-8") once per line
                let fixed = line.replacen(".value", ".payload.data.decode(\"utf-8\")", 1);
                new_lines.push(fixed);
            } else {
                new_lines.push(line.clone());
            }
        }
        result = new_lines.join("\n");
    }

    // --- Cloud KMS patterns ---
    // Only apply when kms is imported
    let has_kms =
        result.contains("from google.cloud import kms") || result.contains("google.cloud.kms");

    if has_kms {
        // response['CiphertextBlob'] → response.ciphertext
        result = result.replace("['CiphertextBlob']", ".ciphertext");
        result = result.replace("[\"CiphertextBlob\"]", ".ciphertext");

        // response['Plaintext'] → response.plaintext
        result = result.replace("['Plaintext']", ".plaintext");
        result = result.replace("[\"Plaintext\"]", ".plaintext");
    }

    // --- Compute Engine patterns ---
    // Only apply when compute_v1 or compute is imported
    let has_compute =
        result.contains("compute_v1") || result.contains("from google.cloud import compute");

    if has_compute {
        // r['Reservations'] → list(r)
        // Walk line by line and replace var['Reservations'] with list(var).
        let lines: Vec<String> = result.lines().map(String::from).collect();
        let mut new_lines = Vec::with_capacity(lines.len());
        for line in &lines {
            let mut fixed = line.clone();
            for quote in &["'", "\""] {
                let pattern = format!("[{quote}Reservations{quote}]");
                if fixed.contains(&pattern) {
                    if let Some(bracket_pos) = fixed.find(&pattern) {
                        let before = &fixed[..bracket_pos];
                        let var_start = before
                            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let var_name = &fixed[var_start..bracket_pos];
                        if !var_name.is_empty() {
                            let old = format!("{var_name}{pattern}");
                            let new_val = format!("list({var_name})");
                            fixed = fixed.replace(&old, &new_val);
                        }
                    }
                }
            }
            new_lines.push(fixed);
        }
        result = new_lines.join("\n");
    }

    // --- Pub/Sub patterns ---
    // Only apply when pubsub_v1 or pubsub is imported
    let has_pubsub =
        result.contains("pubsub_v1") || result.contains("from google.cloud import pubsub");

    if has_pubsub {
        // resp.get('Messages', []) → list(resp.received_messages)
        let lines: Vec<String> = result.lines().map(String::from).collect();
        let mut new_lines = Vec::with_capacity(lines.len());
        for line in &lines {
            let mut fixed = line.clone();
            for quote in &["'", "\""] {
                let pattern = format!(".get({quote}Messages{quote}, [])");
                if fixed.contains(&pattern) {
                    if let Some(dot_pos) = fixed.find(&pattern) {
                        let before = &fixed[..dot_pos];
                        let var_start = before
                            .rfind(|c: char| !c.is_alphanumeric() && c != '_' && c != '.')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let var_name = &fixed[var_start..dot_pos];
                        if !var_name.is_empty() {
                            let old = format!("{var_name}{pattern}");
                            let new_val = format!("list({var_name}.received_messages)");
                            fixed = fixed.replace(&old, &new_val);
                        }
                    }
                }
            }
            new_lines.push(fixed);
        }
        result = new_lines.join("\n");

        // response['MessageId'] → response.result()
        result = result.replace("['MessageId']", ".result()");
        result = result.replace("[\"MessageId\"]", ".result()");
    }

    result
}

/// Fix KMS variable shadowing: `kms = kms.KeyManagementServiceClient()`.
///
/// When `from google.cloud import kms` is present and the code assigns
/// `kms = kms.KeyManagementServiceClient()`, the variable shadows the module.
/// Rename the variable to `kms_client` throughout.
fn fix_kms_variable_shadowing(source: &str) -> String {
    if !source.contains("from google.cloud import kms") {
        return source.to_string();
    }
    if !source.contains("kms = kms.KeyManagementServiceClient()") {
        return source.to_string();
    }

    let mut result = source.to_string();

    // Replace the assignment itself
    result = result.replace(
        "kms = kms.KeyManagementServiceClient()",
        "kms_client = kms.KeyManagementServiceClient()",
    );

    // Replace subsequent uses of `kms.` that refer to the client variable
    // (not the module). After renaming, module-level uses like `kms.CryptoKey`
    // should stay, but method calls like `kms.encrypt(`, `kms.decrypt(` should
    // become `kms_client.encrypt(`, etc.
    //
    // Strategy: walk line by line. After the assignment line, replace `kms.`
    // with `kms_client.` when the token is used as a method call (lower-case
    // method name after the dot), not a class/type reference (upper-case).
    let lines: Vec<String> = result.lines().map(String::from).collect();
    let mut new_lines = Vec::with_capacity(lines.len());
    let mut past_assignment = false;

    for line in &lines {
        if line.contains("kms_client = kms.KeyManagementServiceClient()") {
            past_assignment = true;
            new_lines.push(line.clone());
            continue;
        }

        if past_assignment {
            let fixed = rename_kms_client_refs(line);
            new_lines.push(fixed);
        } else {
            new_lines.push(line.clone());
        }
    }

    new_lines.join("\n")
}

/// Rename `kms.method(` to `kms_client.method(` where method starts lower-case.
/// Preserves module-level references like `kms.CryptoKey`, `kms.KeyRing`.
fn rename_kms_client_refs(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut search_from = 0;
    let bytes = line.as_bytes();

    while search_from < bytes.len() {
        if let Some(pos) = line[search_from..].find("kms.") {
            let abs_pos = search_from + pos;
            // Check word boundary before
            let before_ok = abs_pos == 0
                || (!bytes[abs_pos - 1].is_ascii_alphanumeric() && bytes[abs_pos - 1] != b'_');
            // Check the character after 'kms.' — lowercase = method call on client
            let after_dot = abs_pos + 4;
            let is_method =
                after_dot < bytes.len() && (bytes[after_dot] as char).is_ascii_lowercase();

            if before_ok && is_method {
                result.push_str(&line[search_from..abs_pos]);
                result.push_str("kms_client.");
                search_from = after_dot; // skip past "kms."
            } else {
                result.push_str(&line[search_from..after_dot]);
                search_from = after_dot;
            }
        } else {
            result.push_str(&line[search_from..]);
            break;
        }
    }

    result
}

/// Rename AWS client variable names to idiomatic GCP equivalents.
///
/// After patterns assign GCP clients to AWS-named variables
/// (e.g. `sns = pubsub_v1.PublisherClient()`), we rename the variable
/// throughout the file to avoid confusion.
fn fix_aws_variable_names(source: &str) -> String {
    let mut result = source.to_string();

    // Define (aws_var, gcp_client_constructor_fragment, new_var) triples.
    // We only rename when the specific GCP client constructor is assigned
    // to the old AWS variable name.
    let renames: &[(&str, &str, &str)] = &[
        ("sns", "pubsub_v1.PublisherClient()", "publisher"),
        ("sqs", "pubsub_v1.SubscriberClient()", "subscriber"),
        ("sqs", "pubsub_v1.PublisherClient()", "publisher"),
        ("ses", "pubsub_v1.PublisherClient()", "email_client"),
        (
            "ssm",
            "secretmanager.SecretManagerServiceClient()",
            "sm_client",
        ),
        ("kinesis", "pubsub_v1.PublisherClient()", "publisher"),
    ];

    for &(aws_var, gcp_ctor, new_var) in renames {
        let assignment = format!("{aws_var} = {gcp_ctor}");
        if !result.contains(&assignment) {
            continue;
        }

        // Replace the assignment
        let new_assignment = format!("{new_var} = {gcp_ctor}");
        result = result.replace(&assignment, &new_assignment);

        // Replace subsequent uses of the old variable name at word boundaries
        let lines: Vec<String> = result.lines().map(String::from).collect();
        let mut new_lines = Vec::with_capacity(lines.len());

        for line in &lines {
            // Skip string constant lines — don't rename inside URLs, ARNs, etc.
            let trimmed = line.trim();
            if trimmed.starts_with("QUEUE_URL")
                || trimmed.starts_with("TOPIC_ARN")
                || trimmed.starts_with("STATE_MACHINE")
                || trimmed.starts_with('#')
                || (trimmed.contains("'") && trimmed.contains("://"))
                || (trimmed.contains('"') && trimmed.contains("://"))
                || (trimmed.contains("arn:aws:"))
            {
                new_lines.push(line.clone());
                continue;
            }
            let mut fixed = line.clone();
            // Replace var.xxx patterns (method calls, attribute access)
            let old_dot = format!("{aws_var}.");
            let new_dot = format!("{new_var}.");
            fixed = replace_standalone_token(&fixed, &old_dot, &new_dot);
            // Replace bare variable references at word boundaries
            fixed = replace_bare_variable(&fixed, aws_var, new_var);
            new_lines.push(fixed);
        }

        result = new_lines.join("\n");
    }

    result
}

/// Replace occurrences of `token` with `replacement` only at word boundaries.
/// Ensures we don't replace inside longer identifiers.
fn replace_standalone_token(line: &str, token: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut search_from = 0;
    let bytes = line.as_bytes();
    let token_len = token.len();

    while search_from < bytes.len() {
        if let Some(pos) = line[search_from..].find(token) {
            let abs_pos = search_from + pos;
            // Check that the character before is not alphanumeric or underscore
            let before_ok = abs_pos == 0 || {
                let c = bytes[abs_pos - 1];
                !c.is_ascii_alphanumeric() && c != b'_'
            };
            if before_ok {
                result.push_str(&line[search_from..abs_pos]);
                result.push_str(replacement);
                search_from = abs_pos + token_len;
            } else {
                result.push_str(&line[search_from..abs_pos + 1]);
                search_from = abs_pos + 1;
            }
        } else {
            result.push_str(&line[search_from..]);
            break;
        }
    }

    result
}

/// Replace a bare variable name at word boundaries (not followed by `.`).
/// Handles patterns like `func(var)`, `var,`, `var)`, `= var\n`, etc.
fn replace_bare_variable(line: &str, old_var: &str, new_var: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut search_from = 0;
    let bytes = line.as_bytes();
    let old_len = old_var.len();

    while search_from < bytes.len() {
        if let Some(pos) = line[search_from..].find(old_var) {
            let abs_pos = search_from + pos;
            let end_pos = abs_pos + old_len;

            // Check word boundary before
            let before_ok = abs_pos == 0 || {
                let c = bytes[abs_pos - 1];
                !c.is_ascii_alphanumeric() && c != b'_'
            };
            // Check word boundary after
            let after_ok = end_pos >= bytes.len() || {
                let c = bytes[end_pos];
                !c.is_ascii_alphanumeric() && c != b'_'
            };

            if before_ok && after_ok {
                result.push_str(&line[search_from..abs_pos]);
                result.push_str(new_var);
                search_from = end_pos;
            } else {
                result.push_str(&line[search_from..abs_pos + 1]);
                search_from = abs_pos + 1;
            }
        } else {
            result.push_str(&line[search_from..]);
            break;
        }
    }

    result
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
    // Azure client init leftover tests
    // ---------------------------------------------------------------------------

    #[test]
    fn test_azure_blob_client_init_replaced() {
        let source = r#"from google.cloud import storage
from azure.storage.blob import BlobServiceClient

blob_service = BlobServiceClient.from_connection_string(conn_str)
container = blob_service.get_container_client("my-container")
blob = container.get_blob_client("file.txt")
storage.Client().bucket("my-container").blob("file.txt").upload_from_string(data)
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert!(
            !fixed.contains("BlobServiceClient"),
            "BlobServiceClient should be removed, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("from azure.storage.blob"),
            "Azure import should be removed, got:\n{fixed}"
        );
        assert!(
            fixed.contains("blob_service = storage.Client()"),
            "Should replace with storage.Client(), got:\n{fixed}"
        );
        assert!(
            fixed.contains("container = storage.Client().bucket(\"my-container\")"),
            "Should replace get_container_client, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("get_blob_client"),
            "get_blob_client should be removed, got:\n{fixed}"
        );
        assert!(
            fixed.contains("upload_from_string"),
            "Transformed call should be preserved, got:\n{fixed}"
        );
    }

    #[test]
    fn test_azure_secret_client_replaced() {
        let source = r#"from google.cloud import secretmanager
from azure.keyvault.secrets import SecretClient
from azure.identity import DefaultAzureCredential

client = SecretClient(vault_url="https://myvault.vault.azure.net", credential=DefaultAzureCredential())
secret = client.get_secret("my-secret")
print(secret.value)
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert!(
            !fixed.contains("SecretClient(vault_url"),
            "SecretClient init should be replaced, got:\n{fixed}"
        );
        assert!(
            fixed.contains("client = secretmanager.SecretManagerServiceClient()"),
            "Should replace with SecretManagerServiceClient(), got:\n{fixed}"
        );
        assert!(
            !fixed.contains("from azure.keyvault"),
            "Azure import should be removed, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("DefaultAzureCredential"),
            "DefaultAzureCredential should be removed, got:\n{fixed}"
        );
        assert!(
            fixed.contains(".payload.data.decode(\"utf-8\")"),
            "Should fix .value to .payload.data.decode, got:\n{fixed}"
        );
    }

    #[test]
    fn test_azure_servicebus_client_removed() {
        let source = r#"from google.cloud import pubsub_v1
from azure.servicebus import ServiceBusClient, ServiceBusMessage

client = ServiceBusClient.from_connection_string(conn_str)
sender = client.get_queue_sender("my-queue")
pubsub_v1.PublisherClient().publish(TOPIC_PATH, msg.encode("utf-8"))
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert!(
            !fixed.contains("ServiceBusClient"),
            "ServiceBusClient should be removed, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("get_queue_sender"),
            "get_queue_sender should be removed, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("from azure.servicebus"),
            "Azure import should be removed, got:\n{fixed}"
        );
    }

    #[test]
    fn test_azure_metrics_client_replaced() {
        let source = r#"from google.cloud import monitoring_v3
from azure.monitor.query import MetricsQueryClient
from azure.identity import DefaultAzureCredential

client = MetricsQueryClient(DefaultAzureCredential())
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert!(
            fixed.contains("client = monitoring_v3.MetricServiceClient()"),
            "Should replace MetricsQueryClient, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("MetricsQueryClient"),
            "MetricsQueryClient should be removed, got:\n{fixed}"
        );
    }

    #[test]
    fn test_azure_queue_service_client_replaced() {
        let source = r#"from google.cloud import pubsub_v1
from azure.storage.queue import QueueServiceClient

queue_client = QueueServiceClient.from_connection_string(conn_str)
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert!(
            fixed.contains("queue_client = pubsub_v1.PublisherClient()"),
            "Should replace QueueServiceClient, got:\n{fixed}"
        );
    }

    #[test]
    fn test_azure_cosmos_client_removed() {
        let source = r#"from google.cloud import firestore

client = CosmosClient(url, credential=key)
db = client.get_database_client("app")
container = db.get_container_client("users")
firestore.Client().collection("users").document(item_id).set(item)
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert!(
            !fixed.contains("CosmosClient("),
            "CosmosClient should be removed, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("get_database_client"),
            "get_database_client should be removed, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("get_container_client"),
            "get_container_client should be removed, got:\n{fixed}"
        );
        assert!(
            fixed.contains("firestore.Client()"),
            "Transformed call should be preserved, got:\n{fixed}"
        );
    }

    #[test]
    fn test_azure_eventhub_producer_removed() {
        let source = r#"from google.cloud import pubsub_v1
from azure.eventhub import EventHubProducerClient, EventData

producer = EventHubProducerClient.from_connection_string(conn_str, eventhub_name="events")
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert!(
            !fixed.contains("EventHubProducerClient"),
            "EventHubProducerClient should be removed, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("from azure.eventhub"),
            "Azure eventhub import should be removed, got:\n{fixed}"
        );
    }

    #[test]
    fn test_azure_fixup_noop_without_gcp_imports() {
        // Should not modify anything if no google.cloud imports are present
        let source = r#"from azure.storage.blob import BlobServiceClient

blob_service = BlobServiceClient.from_connection_string(conn_str)
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert_eq!(
            fixed, source,
            "Should not modify code without GCP imports, got:\n{fixed}"
        );
    }

    #[test]
    fn test_azure_fixup_preserves_azure_functions_import() {
        // azure.functions import is handled by a different fixup — don't remove it here
        let source = r#"from google.cloud import storage
import azure.functions as func

storage.Client().bucket("b").blob("k").upload_from_string(data)
"#;
        let fixed = fix_azure_client_init_leftovers(source);
        assert!(
            fixed.contains("azure.functions"),
            "azure.functions import should be preserved, got:\n{fixed}"
        );
    }

    #[test]
    fn test_extract_first_call_arg() {
        assert_eq!(
            extract_first_call_arg(
                "container = blob_service.get_container_client(\"my-bucket\")",
                "get_container_client"
            ),
            Some("\"my-bucket\"".to_string())
        );
        assert_eq!(
            extract_first_call_arg(
                "container = blob_service.get_container_client('bucket', extra)",
                "get_container_client"
            ),
            Some("'bucket'".to_string())
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

    // -----------------------------------------------------------------------
    // AWS response pattern fixup tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_fix_secret_string_pattern() {
        let source = "from google.cloud import secretmanager\nval = resp['SecretString']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains(r#"resp.payload.data.decode("utf-8")"#),
            "Should rewrite SecretString, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("['SecretString']"),
            "Should remove AWS pattern, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_secret_binary_pattern() {
        let source = "from google.cloud import secretmanager\ndata = resp['SecretBinary']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains("resp.payload.data"),
            "Should rewrite SecretBinary, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_parameter_value_pattern() {
        let source = "from google.cloud import secretmanager\nval = r['Parameter']['Value']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains(r#"r.payload.data.decode("utf-8")"#),
            "Should rewrite Parameter/Value, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_secret_string_not_applied_without_import() {
        let source = "val = resp['SecretString']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains("['SecretString']"),
            "Should NOT rewrite without secretmanager import, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_kms_ciphertext_blob() {
        let source = "from google.cloud import kms\nct = response['CiphertextBlob']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains("response.ciphertext"),
            "Should rewrite CiphertextBlob, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_kms_plaintext() {
        let source = "from google.cloud import kms\npt = response['Plaintext']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains("response.plaintext"),
            "Should rewrite Plaintext, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_kms_not_applied_without_import() {
        let source = "ct = response['CiphertextBlob']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains("['CiphertextBlob']"),
            "Should NOT rewrite without kms import, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_reservations_pattern() {
        let source = "from google.cloud import compute_v1\ninstances = r['Reservations']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains("list(r)"),
            "Should rewrite Reservations to list(), got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_pubsub_messages_pattern() {
        let source = "from google.cloud import pubsub_v1\nmsgs = resp.get('Messages', [])";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains("list(resp.received_messages)"),
            "Should rewrite Messages to received_messages, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_pubsub_message_id_pattern() {
        let source = "from google.cloud import pubsub_v1\nmid = response['MessageId']";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains("response.result()"),
            "Should rewrite MessageId to .result(), got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_secret_value_attribute() {
        let source = "from google.cloud import secretmanager\nval = secret_response.value";
        let fixed = fix_aws_response_patterns(source);
        assert!(
            fixed.contains(r#".payload.data.decode("utf-8")"#),
            "Should rewrite .value on secret response, got:\n{fixed}"
        );
    }

    // -----------------------------------------------------------------------
    // KMS variable shadowing fixup tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_fix_kms_variable_shadowing() {
        let source = "from google.cloud import kms\nkms = kms.KeyManagementServiceClient()\nresult = kms.encrypt(request={\"name\": key_name})\nkey = kms.CryptoKey(name=\"test\")";
        let fixed = fix_kms_variable_shadowing(source);
        assert!(
            fixed.contains("kms_client = kms.KeyManagementServiceClient()"),
            "Should rename assignment, got:\n{fixed}"
        );
        assert!(
            fixed.contains("kms_client.encrypt("),
            "Should rename method calls, got:\n{fixed}"
        );
        assert!(
            fixed.contains("kms.CryptoKey("),
            "Should preserve module-level class access, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_kms_shadowing_noop_without_import() {
        let source = "kms = kms.KeyManagementServiceClient()";
        let fixed = fix_kms_variable_shadowing(source);
        assert_eq!(fixed, source, "Should not modify without import");
    }

    // -----------------------------------------------------------------------
    // AWS variable name renaming tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_fix_sns_to_publisher() {
        let source = "from google.cloud import pubsub_v1\nsns = pubsub_v1.PublisherClient()\nresult = sns.publish(topic_path, data=message)\nprint(sns)";
        let fixed = fix_aws_variable_names(source);
        assert!(
            fixed.contains("publisher = pubsub_v1.PublisherClient()"),
            "Should rename assignment, got:\n{fixed}"
        );
        assert!(
            fixed.contains("publisher.publish("),
            "Should rename method calls, got:\n{fixed}"
        );
        assert!(
            fixed.contains("print(publisher)"),
            "Should rename bare references, got:\n{fixed}"
        );
        assert!(
            !fixed.contains("sns"),
            "Should not contain old variable name, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_ssm_to_sm_client() {
        let source = "ssm = secretmanager.SecretManagerServiceClient()\nval = ssm.access_secret_version(name=secret_name)";
        let fixed = fix_aws_variable_names(source);
        assert!(
            fixed.contains("sm_client = secretmanager.SecretManagerServiceClient()"),
            "Should rename ssm to sm_client, got:\n{fixed}"
        );
        assert!(
            fixed.contains("sm_client.access_secret_version("),
            "Should rename method calls, got:\n{fixed}"
        );
    }

    #[test]
    fn test_fix_aws_var_noop_without_match() {
        let source = "client = storage.Client()\nresult = client.list_buckets()";
        let fixed = fix_aws_variable_names(source);
        assert_eq!(fixed, source, "Should not modify non-matching patterns");
    }

    #[test]
    fn test_fix_aws_var_no_substring_replacement() {
        let source = "from google.cloud import pubsub_v1\nsns = pubsub_v1.PublisherClient()\nsns_topic_name = \"projects/p/topics/t\"\nresult = sns.publish(sns_topic_name, data=message)";
        let fixed = fix_aws_variable_names(source);
        assert!(
            fixed.contains("sns_topic_name"),
            "Should NOT rename sns inside sns_topic_name, got:\n{fixed}"
        );
        assert!(
            fixed.contains("publisher.publish("),
            "Should rename standalone sns, got:\n{fixed}"
        );
    }
}
