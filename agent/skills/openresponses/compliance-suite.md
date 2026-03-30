# Open Responses Integration Tests

Below is a passing example of the open responses complience test suite

## Basic Text Response

Simple user message, validates ResponseResource schema.

### Input

```json
{
  "model": "gpt-4o-mini",
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": "Say hello in exactly 3 words."
    }
  ]
}
```

### Output

```json
{
  "id": "resp_0969b02502f499e700697055b59cbc8190b9667eefb50289c3",
  "object": "response",
  "created_at": 1768969653,
  "completed_at": 1768969655,
  "status": "completed",
  "incomplete_details": null,
  "model": "gpt-4o-mini-2024-07-18",
  "previous_response_id": null,
  "instructions": null,
  "output": [
    {
      "type": "message",
      "id": "msg_0969b02502f499e700697055b7750c8190a7847d642272dba1",
      "status": "completed",
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "Hello, how are you?",
          "annotations": [],
          "logprobs": []
        }
      ]
    }
  ],
  "error": null,
  "tools": [],
  "tool_choice": "auto",
  "truncation": "disabled",
  "parallel_tool_calls": true,
  "text": {
    "format": {
      "type": "text"
    },
    "verbosity": "medium"
  },
  "top_p": 1,
  "presence_penalty": 0,
  "frequency_penalty": 0,
  "top_logprobs": 0,
  "temperature": 1,
  "reasoning": {
    "effort": null,
    "summary": null
  },
  "usage": {
    "input_tokens": 15,
    "output_tokens": 7,
    "total_tokens": 22,
    "input_tokens_details": {
      "cached_tokens": 0
    },
    "output_tokens_details": {
      "reasoning_tokens": 0
    }
  },
  "max_output_tokens": null,
  "max_tool_calls": null,
  "store": true,
  "background": false,
  "service_tier": "default",
  "metadata": {},
  "safety_identifier": null,
  "prompt_cache_key": null
}
```

---

## Streaming Response

Validates SSE streaming events and final response.

### Input

```json
{
  "model": "gpt-4o-mini",
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": "Count from 1 to 5."
    }
  ],
  "stream": true
}
```

### Output

```json
{
  "id": "resp_00fdad8a4a3104ce00697055b505988195be4653f46e337f01",
  "object": "response",
  "created_at": 1768969653,
  "completed_at": 1768969653,
  "status": "completed",
  "incomplete_details": null,
  "model": "gpt-4o-mini-2024-07-18",
  "previous_response_id": null,
  "instructions": null,
  "output": [
    {
      "type": "message",
      "id": "msg_00fdad8a4a3104ce00697055b5533881959fdebfaf6eff259f",
      "status": "completed",
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "1, 2, 3, 4, 5.",
          "annotations": [],
          "logprobs": []
        }
      ]
    }
  ],
  "error": null,
  "tools": [],
  "tool_choice": "auto",
  "truncation": "disabled",
  "parallel_tool_calls": true,
  "text": {
    "format": {
      "type": "text"
    },
    "verbosity": "medium"
  },
  "top_p": 1,
  "presence_penalty": 0,
  "frequency_penalty": 0,
  "top_logprobs": 0,
  "temperature": 1,
  "reasoning": {
    "effort": null,
    "summary": null
  },
  "usage": {
    "input_tokens": 15,
    "output_tokens": 15,
    "total_tokens": 30,
    "input_tokens_details": {
      "cached_tokens": 0
    },
    "output_tokens_details": {
      "reasoning_tokens": 0
    }
  },
  "max_output_tokens": null,
  "max_tool_calls": null,
  "store": true,
  "background": false,
  "service_tier": "default",
  "metadata": {},
  "safety_identifier": null,
  "prompt_cache_key": null
}
```

---

## System Prompt

Include system role message in input.

### Input

```json
{
  "model": "gpt-4o-mini",
  "input": [
    {
      "type": "message",
      "role": "system",
      "content": "You are a pirate. Always respond in pirate speak."
    },
    {
      "type": "message",
      "role": "user",
      "content": "Say hello."
    }
  ]
}
```

### Output

```json
{
  "id": "resp_06701671aff95dbf00697055b508ac81a3afdb5a249bebe86e",
  "object": "response",
  "created_at": 1768969653,
  "completed_at": 1768969654,
  "status": "completed",
  "incomplete_details": null,
  "model": "gpt-4o-mini-2024-07-18",
  "previous_response_id": null,
  "instructions": null,
  "output": [
    {
      "type": "message",
      "id": "msg_06701671aff95dbf00697055b558d881a3a9e284b714ced4da",
      "status": "completed",
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "Ahoy, me hearty! Greetings to ye on this fine day upon the high seas! What brings ye to me domain?",
          "annotations": [],
          "logprobs": []
        }
      ]
    }
  ],
  "error": null,
  "tools": [],
  "tool_choice": "auto",
  "truncation": "disabled",
  "parallel_tool_calls": true,
  "text": {
    "format": {
      "type": "text"
    },
    "verbosity": "medium"
  },
  "top_p": 1,
  "presence_penalty": 0,
  "frequency_penalty": 0,
  "top_logprobs": 0,
  "temperature": 1,
  "reasoning": {
    "effort": null,
    "summary": null
  },
  "usage": {
    "input_tokens": 25,
    "output_tokens": 26,
    "total_tokens": 51,
    "input_tokens_details": {
      "cached_tokens": 0
    },
    "output_tokens_details": {
      "reasoning_tokens": 0
    }
  },
  "max_output_tokens": null,
  "max_tool_calls": null,
  "store": true,
  "background": false,
  "service_tier": "default",
  "metadata": {},
  "safety_identifier": null,
  "prompt_cache_key": null
}
```

---

## Tool Calling

Define a function tool and verify function_call output.

### Input

```json
{
  "model": "gpt-4o-mini",
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": "What's the weather like in San Francisco?"
    }
  ],
  "tools": [
    {
      "type": "function",
      "name": "get_weather",
      "description": "Get the current weather for a location",
      "parameters": {
        "type": "object",
        "properties": {
          "location": {
            "type": "string",
            "description": "The city and state, e.g. San Francisco, CA"
          }
        },
        "required": [
          "location"
        ]
      }
    }
  ]
}
```

### Output

```json
{
  "id": "resp_0633efef9be07b3f00697055b509f481919987db82fc1022c9",
  "object": "response",
  "created_at": 1768969653,
  "completed_at": 1768969653,
  "status": "completed",
  "incomplete_details": null,
  "model": "gpt-4o-mini-2024-07-18",
  "previous_response_id": null,
  "instructions": null,
  "output": [
    {
      "type": "function_call",
      "id": "fc_0633efef9be07b3f00697055b5992081919d798950dd078737",
      "call_id": "call_In5iIk9f6NxYKBb54Jd1tMmI",
      "name": "get_weather",
      "arguments": "{\"location\":\"San Francisco, CA\"}",
      "status": "completed"
    }
  ],
  "error": null,
  "tools": [
    {
      "type": "function",
      "name": "get_weather",
      "description": "Get the current weather for a location",
      "parameters": {
        "type": "object",
        "properties": {
          "location": {
            "type": "string",
            "description": "The city and state, e.g. San Francisco, CA"
          }
        },
        "required": [
          "location"
        ],
        "additionalProperties": false
      },
      "strict": true
    }
  ],
  "tool_choice": "auto",
  "truncation": "disabled",
  "parallel_tool_calls": true,
  "text": {
    "format": {
      "type": "text"
    },
    "verbosity": "medium"
  },
  "top_p": 1,
  "presence_penalty": 0,
  "frequency_penalty": 0,
  "top_logprobs": 0,
  "temperature": 1,
  "reasoning": {
    "effort": null,
    "summary": null
  },
  "usage": {
    "input_tokens": 61,
    "output_tokens": 18,
    "total_tokens": 79,
    "input_tokens_details": {
      "cached_tokens": 0
    },
    "output_tokens_details": {
      "reasoning_tokens": 0
    }
  },
  "max_output_tokens": null,
  "max_tool_calls": null,
  "store": true,
  "background": false,
  "service_tier": "default",
  "metadata": {},
  "safety_identifier": null,
  "prompt_cache_key": null
}
```

---

## Image Input

Send image URL in user content.

### Input

```json
{
  "model": "gpt-4o-mini",
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": [
        {
          "type": "input_text",
          "text": "What do you see in this image? Answer in one sentence."
        },
        {
          "type": "input_image",
          "image_url": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAACAAAAAgCAIAAAD8GO2jAAABmklEQVR42tyWAaTyUBzFew/eG4AHz+MBSAHKBiJRGFKwIgQQJKLUIioBIhCAiCAAEizAQIAECaASqFFJq84nudjnaqvuPnxzgP9xfrq5938csPn7PwHTKSoViCIEAYEAMhmoKsU2mUCWEQqB5xEMIp/HaGQG2G6RSuH9HQ7H34rFrtPbdz4jl6PbwmEsl3QA1mt4vcRKk8dz9eg6IpF7tt9fzGY0gCgafFRFo5Blc5vLhf3eCOj1yNhM5GRMVK0aATxPZoz09YXjkQDmczJgquGQAPp9WwCNBgG027YACgUC6HRsAZRKBDAY2AJoNv/ZnwzA6WScznG3p4UAymXGAEkyXrTFAh8fLAGqagQAyGaZpYsi7bHTNPz8MEj//LxuFPo+UBS8vb0KaLXubrRa7aX0RMLCykwmn0z3+XA4WACcTpCkh9MFAZpmuVXo+mO/w+/HZvNgbblcUCxaSo/Hyck80Yu6XXDcvfVZr79cvMZjuN2U9O9vKAqjZrfbIZ0mV4TUi9Xqz6jddNy//7+e3n8Fhf/Llo2kxi8AQyGRoDkmAhAAAAAASUVORK5CYII="
        }
      ]
    }
  ]
}
```

### Output

```json
{
  "id": "resp_00e03e5c6b6eb62800697055b50b548191b4511609c580303f",
  "object": "response",
  "created_at": 1768969653,
  "completed_at": 1768969653,
  "status": "completed",
  "incomplete_details": null,
  "model": "gpt-4o-mini-2024-07-18",
  "previous_response_id": null,
  "instructions": null,
  "output": [
    {
      "type": "message",
      "id": "msg_00e03e5c6b6eb62800697055b578008191a5f210dd5dc16d57",
      "status": "completed",
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "I see a red heart symbol.",
          "annotations": [],
          "logprobs": []
        }
      ]
    }
  ],
  "error": null,
  "tools": [],
  "tool_choice": "auto",
  "truncation": "disabled",
  "parallel_tool_calls": true,
  "text": {
    "format": {
      "type": "text"
    },
    "verbosity": "medium"
  },
  "top_p": 1,
  "presence_penalty": 0,
  "frequency_penalty": 0,
  "top_logprobs": 0,
  "temperature": 1,
  "reasoning": {
    "effort": null,
    "summary": null
  },
  "usage": {
    "input_tokens": 8520,
    "output_tokens": 8,
    "total_tokens": 8528,
    "input_tokens_details": {
      "cached_tokens": 0
    },
    "output_tokens_details": {
      "reasoning_tokens": 0
    }
  },
  "max_output_tokens": null,
  "max_tool_calls": null,
  "store": true,
  "background": false,
  "service_tier": "default",
  "metadata": {},
  "safety_identifier": null,
  "prompt_cache_key": null
}
```

---

## Multi-turn Conversation

Send assistant + user messages as conversation history.

### Input

```json
{
  "model": "gpt-4o-mini",
  "input": [
    {
      "type": "message",
      "role": "user",
      "content": "My name is Alice."
    },
    {
      "type": "message",
      "role": "assistant",
      "content": "Hello Alice! Nice to meet you. How can I help you today?"
    },
    {
      "type": "message",
      "role": "user",
      "content": "What is my name?"
    }
  ]
}
```

### Output

```json
{
  "id": "resp_06c22c5742d1ec3d00697055b50d70819cab1beb3b3edefa4b",
  "object": "response",
  "created_at": 1768969653,
  "completed_at": 1768969653,
  "status": "completed",
  "incomplete_details": null,
  "model": "gpt-4o-mini-2024-07-18",
  "previous_response_id": null,
  "instructions": null,
  "output": [
    {
      "type": "message",
      "id": "msg_06c22c5742d1ec3d00697055b54064819ca3489eaa9bac827d",
      "status": "completed",
      "role": "assistant",
      "content": [
        {
          "type": "output_text",
          "text": "Your name is Alice! How can I assist you today?",
          "annotations": [],
          "logprobs": []
        }
      ]
    }
  ],
  "error": null,
  "tools": [],
  "tool_choice": "auto",
  "truncation": "disabled",
  "parallel_tool_calls": true,
  "text": {
    "format": {
      "type": "text"
    },
    "verbosity": "medium"
  },
  "top_p": 1,
  "presence_penalty": 0,
  "frequency_penalty": 0,
  "top_logprobs": 0,
  "temperature": 1,
  "reasoning": {
    "effort": null,
    "summary": null
  },
  "usage": {
    "input_tokens": 40,
    "output_tokens": 13,
    "total_tokens": 53,
    "input_tokens_details": {
      "cached_tokens": 0
    },
    "output_tokens_details": {
      "reasoning_tokens": 0
    }
  },
  "max_output_tokens": null,
  "max_tool_calls": null,
  "store": true,
  "background": false,
  "service_tier": "default",
  "metadata": {},
  "safety_identifier": null,
  "prompt_cache_key": null
}
```
