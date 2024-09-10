# Translation Performance Analysis

## Current Data
I've obtained data using the script `bin/measure_translation_time.rs`. All the data is in `tests/benchmark/bench.csv`.

## Observations
1. The duration per character decreases as the number of characters increases, which is expected. It seems to get as low as 1.5ms per character. 500k * 1.5ms = 750s = 12.5 minutes.
2. The limit of one single request is 128 KiB, around 130k characters. https://developers.deepl.com/docs/api-reference/translate
3. We would need 750 concurrent requests to translate 500k characters in under 1 second.
4. The actual duration per request is more than 300ms, so we can send up to 3 requests per thread. With 750 concurrent threads, we can send up to 3 * 750 = 2250 requests per second. To translate 500k characters in one second, we would need approximately 500k / 2250 ≈ 222 characters per request.


## Potential Improvements

1. Test how many concurrent request DeepL allow us to use per key.
   - Rationale: [Why you think this might help]
   - Implementation: [Brief notes on how to implement]

2. Use the batch api if there are too many message.
   - Rationale: [Why you think this might help]
   - Implementation: [Brief notes on how to implement]

3. Custom batching,
   - Rationale: If the actual batching endpoint of DeepL is slower, we could join Strings with an special character and then separate them back.
   - Implementation: Use an special character to join them like ~, | or `.O.`, something that won't make the deepL model panic or change the maining of the text.

## Next Steps

1. Implement concurrent request testing to determine DeepL's limits.
2. Build a mock server that emulates the DeepL performance, same number of threads and processing time.
2. Compare performance between single requests and batch API.
3. Develop and test custom batching solution if needed.

In short, maximize threads, reduce number of messages per thread. The theoretical minimum would be one message per thread with as many threads as DeepL allows.
