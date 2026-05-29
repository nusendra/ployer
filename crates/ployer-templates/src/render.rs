use std::collections::HashMap;

use rand::{distributions::Alphanumeric, Rng, RngCore};

use crate::error::TemplateError;
use crate::schema::{Generate, GenerateKind, Input, InputKind, Template};

pub struct RenderContext<'a> {
    pub app_name: &'a str,
    pub app_id: &'a str,
    pub network: &'a str,
    pub inputs: HashMap<String, String>,
}

pub struct Rendered {
    pub compose: String,
    pub resolved_inputs: HashMap<String, String>,
    pub post_install_message: Option<String>,
    pub outputs: Vec<(String, String)>,
}

pub fn render(template: &Template, ctx: RenderContext) -> Result<Rendered, TemplateError> {
    let resolved = resolve_inputs(&template.inputs, ctx.inputs)?;

    let volumes: HashMap<String, String> = template
        .volumes
        .iter()
        .map(|v| (v.name.clone(), format!("ployer_{}_{}", ctx.app_name, v.name)))
        .collect();

    let compose = substitute(&template.compose, &resolved, &volumes, ctx.app_name, ctx.app_id, ctx.network);

    let (post_install_message, outputs) = match &template.post_install {
        Some(pi) => {
            let message = pi
                .message
                .as_ref()
                .map(|m| substitute(m, &resolved, &volumes, ctx.app_name, ctx.app_id, ctx.network));
            let outputs = pi
                .outputs
                .iter()
                .map(|o| {
                    (
                        o.label.clone(),
                        substitute(&o.value, &resolved, &volumes, ctx.app_name, ctx.app_id, ctx.network),
                    )
                })
                .collect();
            (message, outputs)
        }
        None => (None, Vec::new()),
    };

    Ok(Rendered {
        compose,
        resolved_inputs: resolved,
        post_install_message,
        outputs,
    })
}

fn resolve_inputs(
    inputs: &[Input],
    mut provided: HashMap<String, String>,
) -> Result<HashMap<String, String>, TemplateError> {
    let mut out = HashMap::new();
    for spec in inputs {
        let value = if let Some(v) = provided.remove(&spec.key) {
            if v.is_empty() {
                fallback_value(spec)?
            } else {
                v
            }
        } else {
            fallback_value(spec)?
        };

        if matches!(spec.kind, InputKind::Number) {
            if value.parse::<f64>().is_err() {
                return Err(TemplateError::InvalidInput {
                    key: spec.key.clone(),
                    reason: "expected number".to_string(),
                });
            }
        }

        out.insert(spec.key.clone(), value);
    }
    Ok(out)
}

fn fallback_value(spec: &Input) -> Result<String, TemplateError> {
    if let Some(g) = &spec.generate {
        return Ok(generate(g));
    }
    if let Some(d) = &spec.default {
        return Ok(d.clone());
    }
    Err(TemplateError::MissingInput(spec.key.clone()))
}

fn generate(g: &Generate) -> String {
    let len = g.length.unwrap_or(32);
    match g.kind {
        GenerateKind::Password => {
            let mut rng = rand::thread_rng();
            (0..len).map(|_| rng.sample(Alphanumeric) as char).collect()
        }
        GenerateKind::Hex => {
            let mut bytes = vec![0u8; len.div_ceil(2)];
            rand::thread_rng().fill_bytes(&mut bytes);
            hex::encode(&bytes)[..len].to_string()
        }
    }
}

/// Minimal `{{ var }}` substitution. Supports:
///   inputs.KEY, volumes.NAME, app.name, app.id, network
fn substitute(
    src: &str,
    inputs: &HashMap<String, String>,
    volumes: &HashMap<String, String>,
    app_name: &str,
    app_id: &str,
    network: &str,
) -> String {
    let mut out = String::with_capacity(src.len());
    let mut rest = src;

    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = match after.find("}}") {
            Some(e) => e,
            None => {
                out.push_str("{{");
                rest = after;
                continue;
            }
        };
        let expr = after[..end].trim();
        let resolved = match expr.split_once('.') {
            Some(("inputs", key)) => inputs.get(key).cloned(),
            Some(("volumes", name)) => volumes.get(name).cloned(),
            Some(("app", "name")) => Some(app_name.to_string()),
            Some(("app", "id")) => Some(app_id.to_string()),
            _ if expr == "network" => Some(network.to_string()),
            _ => None,
        };
        match resolved {
            Some(v) => out.push_str(&v),
            None => {
                out.push_str("{{");
                out.push_str(expr);
                out.push_str("}}");
            }
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}
