# Script to translate text using the DeepL API
curl -X POST 'https://api-free.deepl.com/v2/translate' \
--header "Authorization: DeepL-Auth-Key $DEEPL_API_KEY" \
--header 'Content-Type: application/json' \
--data '{
  "text": [
    "Hello, world!"
  ],
  "target_lang": "ES"
}'