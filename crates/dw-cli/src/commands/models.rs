use dw_client::DwClient;
use dw_client::types::models::ModelResponse;

use crate::output::{Displayable, OutputFormat, print_item, print_list};

impl Displayable for ModelResponse {
    fn table_headers() -> Vec<&'static str> {
        vec!["Alias", "Model", "Type", "Capabilities"]
    }

    fn to_table_row(&self) -> Vec<String> {
        vec![
            self.alias.clone(),
            self.model_name.clone(),
            self.model_type.clone().unwrap_or_default(),
            self.capabilities
                .as_ref()
                .map(|c| c.join(", "))
                .unwrap_or_default(),
        ]
    }

    fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or_default()
    }

    fn to_plain(&self) -> String {
        self.alias.clone()
    }
}

pub async fn list(
    client: &DwClient,
    type_filter: Option<&str>,
    format: OutputFormat,
) -> anyhow::Result<()> {
    let response = client.list_models().await?;
    let mut models = response.data;

    if let Some(filter) = type_filter {
        models.retain(|m| {
            m.model_type
                .as_deref()
                .is_some_and(|t| t.eq_ignore_ascii_case(filter))
        });
    }

    print_list(&models, format);
    Ok(())
}

pub async fn get(client: &DwClient, model_id: &str, format: OutputFormat) -> anyhow::Result<()> {
    let model = client.get_model(model_id).await?;
    print_item(&model, format);
    Ok(())
}
