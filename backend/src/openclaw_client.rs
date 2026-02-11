use crate::models::NewsletterConfig;
use reqwest::Client;
use serde_json::Value;

#[derive(Clone)]
pub struct OpenClawConfig {
    pub gateway_url: String,
    pub token: String,
    pub agent_id: String,
}

pub async fn generate_newsletter(
    client: &Client,
    config: &OpenClawConfig,
    newsletter: &NewsletterConfig,
) -> Result<String, String> {
    if config.gateway_url.is_empty() {
        return Err("OpenClaw not configured (OPENCLAW_GATEWAY_URL empty). Set it in .env when ready.".into());
    }
    let prompt = build_prompt(newsletter);
    let body = serde_json::json!({
        "model": format!("openclaw:{}", config.agent_id),
        "input": [
            {
                "type": "message",
                "role": "user",
                "content": prompt
            }
        ],
        "instructions": "You are a newsletter writer. Produce a single newsletter document. Include only the sections the user requested. Follow their per-section instructions. Output plain text or markdown suitable for email."
    });

    let res = client
        .post(&config.gateway_url)
        .header("Authorization", format!("Bearer {}", config.token))
        .header("x-openclaw-agent-id", &config.agent_id)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        let status = res.status();
        let text = res.text().await.unwrap_or_default();
        return Err(format!("OpenClaw HTTP {}: {}", status, text));
    }

    let json: Value = res.json().await.map_err(|e| e.to_string())?;
    let output = json
        .get("output")
        .and_then(|o| o.as_array())
        .and_then(|arr| {
            arr.iter()
                .find(|i| i.get("type").and_then(|t| t.as_str()) == Some("message"))
        })
        .and_then(|m| m.get("content"));
    let text = match output {
        Some(Value::Array(content_parts)) => content_parts
            .iter()
            .filter_map(|p| {
                if p.get("type").and_then(|t| t.as_str()) == Some("output_text") {
                    p.get("text").and_then(|t| t.as_str()).map(String::from)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    };
    Ok(text)
}

fn build_prompt(config: &NewsletterConfig) -> String {
    let mut parts = vec![
        format!("Write a daily newsletter with title: {}", config.title),
        format!("Topics: {}", config.topics.join(", ")),
        format!("Tone: {}", config.tone),
        format!("Length: {}", config.length),
    ];
    let features = config.features.as_object();
    if let Some(feats) = features {
        for (key, v) in feats {
            let enabled = v.get("enabled").and_then(|e| e.as_bool()).unwrap_or(false);
            if !enabled {
                continue;
            }
            let custom: String = v
                .get("custom_request")
                .and_then(|c| c.as_str())
                .unwrap_or("")
                .to_string();
            let label = key.replace('_', " ");
            let label = label.split_whitespace().map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().chain(c).collect(),
                }
            }).collect::<Vec<_>>().join(" ");
            if custom.is_empty() {
                parts.push(format!("Include a section: {}.", label));
            } else {
                parts.push(format!("Include a section: {}. User instructions for this section: {}", label, custom));
            }
        }
    }
    parts.join("\n")
}
