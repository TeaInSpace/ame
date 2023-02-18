/*

static AME_SECRET_STORE: &str = "ame";
impl From<AmeSecret> for Secret {
    fn from(ame_secret: AmeSecret) -> Self {
        let mut secret_map = BTreeMap::new();
        secret_map.insert("secret".to_string(), ame_secret.value);

        let mut labels = BTreeMap::new();
        labels.insert("SECRET_STORE".to_string(), AME_SECRET_STORE.to_string());

        let mut secret = Secret {
            metadata: ObjectMeta {
                name: Some(key.to_string()),
                labels: Some(labels),
                ..ObjectMeta::default()
            },
            string_data: Some(secret_map),
            ..Secret::default()
        };
    }
}*/
