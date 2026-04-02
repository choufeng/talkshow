use std::collections::HashMap;

const SERVICE_NAME: &str = "com.jiaxia.talkshow";

pub fn store_api_key(provider_id: &str, api_key: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
    if api_key.is_empty() {
        entry
            .delete_credential()
            .map_err(|e| format!("Failed to delete keyring entry: {}", e))?;
    } else {
        entry
            .set_password(api_key)
            .map_err(|e| format!("Failed to store API key: {}", e))?;
    }
    Ok(())
}

pub fn get_api_key(provider_id: &str) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
    match entry.get_password() {
        Ok(key) => Ok(Some(key)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(format!("Failed to get API key: {}", e)),
    }
}

#[allow(dead_code)]
pub fn delete_api_key(provider_id: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, provider_id)
        .map_err(|e| format!("Failed to create keyring entry: {}", e))?;
    entry
        .delete_credential()
        .map_err(|e| format!("Failed to delete API key: {}", e))?;
    Ok(())
}

pub fn load_all_api_keys(provider_ids: &[String]) -> HashMap<String, String> {
    let mut keys = HashMap::new();
    for id in provider_ids {
        if let Ok(Some(key)) = get_api_key(id) {
            keys.insert(id.clone(), key);
        }
    }
    keys
}
