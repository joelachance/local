package sdk

// QueryOpts configures the Query call.
type QueryOpts struct {
	TopK        int  `json:"-"`
	UseReranker bool `json:"use_reranker"`
	Raw         bool `json:"raw"`
}

// ConversationMessage represents a single turn in a conversation.
type ConversationMessage struct {
	Role    string `json:"role"`
	Content string `json:"content"`
}
