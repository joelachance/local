package sdk_test

import (
	"context"

	memkit "memkit.dev/sdk"
)

func ExampleMemkit() {
	model, tools := memkit.Memkit("gpt-4")
	_ = model
	_ = tools
	// model == "gpt-4", tools in OpenAI format
}

func ExampleMemkit_anthropic() {
	model, tools := memkit.Memkit("claude-3")
	_ = model
	_ = tools
	// tools in Anthropic input_schema format
}

func ExampleConfigure() {
	memkit.Configure("http://localhost:4242")
}

func ExampleQuery() {
	ctx := context.Background()
	memkit.Configure("http://localhost:4242")
	result, err := memkit.Query(ctx, "what is X?", nil)
	if err != nil {
		return
	}
	_ = result
}

func ExampleAdd_string() {
	ctx := context.Background()
	_ = memkit.Add(ctx, "inline document content")
}

func ExampleAdd_strings() {
	ctx := context.Background()
	_ = memkit.Add(ctx, []string{"doc1", "doc2"})
}

func ExampleAdd_conversation() {
	ctx := context.Background()
	_ = memkit.Add(ctx, []memkit.ConversationMessage{
		{Role: "user", Content: "hello"},
		{Role: "assistant", Content: "hi"},
	})
}

func ExampleExecuteTool() {
	ctx := context.Background()
	result, err := memkit.ExecuteTool(ctx, "memory_status", map[string]any{})
	if err != nil {
		return
	}
	_ = result
}
