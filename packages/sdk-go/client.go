package sdk

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"strings"
	"sync"
	"time"
)

const defaultURL = "https://api.memkit.io"
const timeout = 120 * time.Second

var (
	configURL string
	configMu  sync.RWMutex
)

func init() {
	if u := os.Getenv("MEMKIT_URL"); u != "" {
		configURL = strings.TrimRight(u, "/")
	} else {
		configURL = defaultURL
	}
}

func getConfigURL() string {
	configMu.RLock()
	defer configMu.RUnlock()
	return configURL
}

func setConfigURL(url string) {
	if url == "" {
		return
	}
	configMu.Lock()
	defer configMu.Unlock()
	configURL = strings.TrimRight(url, "/")
}

func clientGet(ctx context.Context, path string) (map[string]any, error) {
	base := getConfigURL()
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, base+path, nil)
	if err != nil {
		return nil, err
	}
	client := &http.Client{Timeout: timeout}
	resp, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("memkit: %d %s", resp.StatusCode, string(body))
	}
	var out map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return nil, err
	}
	return out, nil
}

func clientPost(ctx context.Context, path string, body any) (map[string]any, error) {
	base := getConfigURL()
	raw, err := json.Marshal(body)
	if err != nil {
		return nil, err
	}
	req, err := http.NewRequestWithContext(ctx, http.MethodPost, base+path, bytes.NewReader(raw))
	if err != nil {
		return nil, err
	}
	req.Header.Set("Content-Type", "application/json")
	client := &http.Client{Timeout: timeout}
	resp, err := client.Do(req)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		body, _ := io.ReadAll(resp.Body)
		return nil, fmt.Errorf("memkit: %d %s", resp.StatusCode, string(body))
	}
	var out map[string]any
	if err := json.NewDecoder(resp.Body).Decode(&out); err != nil {
		return nil, err
	}
	return out, nil
}
