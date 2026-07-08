// Package clipdata provides CLIP ViT-B/32 BPE tokenization assets and encoding.
package clipdata

import (
	_ "embed"
	"encoding/json"
	"fmt"
	"strings"
	"sync"
)

const (
	StartOfText = 49406
	EndOfText   = 49407
	MaxTokens   = 77
	FeatureDim  = 512
)

//go:embed vocab.json
var vocabJSON []byte

//go:embed merges.txt
var mergesTXT []byte

var (
	tokenizerOnce sync.Once
	tokenizer     *bpeTokenizer
	tokenizerErr  error
)

type bpeTokenizer struct {
	byteEncoder map[byte]rune
	byteDecoder map[rune]byte
	bpeRanks    map[string]int
	tokenToID   map[string]int
}

func getTokenizer() (*bpeTokenizer, error) {
	tokenizerOnce.Do(func() {
		tokenizer, tokenizerErr = newBPETokenizer()
	})
	return tokenizer, tokenizerErr
}

func newBPETokenizer() (*bpeTokenizer, error) {
	t := &bpeTokenizer{
		byteEncoder: bytesToUnicode(),
		tokenToID:   make(map[string]int),
		bpeRanks:    make(map[string]int),
	}
	t.byteDecoder = make(map[rune]byte, len(t.byteEncoder))
	for b, r := range t.byteEncoder {
		t.byteDecoder[r] = b
	}
	if err := json.Unmarshal(vocabJSON, &t.tokenToID); err != nil {
		return nil, fmt.Errorf("parse vocab: %w", err)
	}
	lines := strings.Split(string(mergesTXT), "\n")
	rank := 0
	for _, line := range lines[1:] { // skip version header
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		t.bpeRanks[line] = rank
		rank++
	}
	return t, nil
}

// EncodeTokenIDs tokenizes text into CLIP input_ids and attention_mask (length 77).
func EncodeTokenIDs(text string) (ids, mask []int64, err error) {
	tok, err := getTokenizer()
	if err != nil {
		return nil, nil, err
	}
	tokens := tok.encode(text)
	ids = make([]int64, MaxTokens)
	mask = make([]int64, MaxTokens)
	ids[0] = StartOfText
	mask[0] = 1
	pos := 1
	for _, token := range tokens {
		if pos >= MaxTokens-1 {
			break
		}
		id, ok := tok.tokenToID[token]
		if !ok {
			id = tok.tokenToID["<|unk|>"]
		}
		ids[pos] = int64(id)
		mask[pos] = 1
		pos++
	}
	ids[pos] = EndOfText
	mask[pos] = 1
	return ids, mask, nil
}

func (t *bpeTokenizer) encode(text string) []string {
	text = collapseWhitespace(strings.ToLower(strings.TrimSpace(text)))
	if text == "" {
		return nil
	}
	var bpeTokens []string
	for _, part := range splitCLIPWords(text) {
		if part == "" {
			continue
		}
		var tokenChars []rune
		for _, r := range part {
			tokenChars = append(tokenChars, t.byteEncoder[byte(r)])
		}
		word := string(tokenChars)
		for _, tok := range t.bpe(word) {
			bpeTokens = append(bpeTokens, tok)
		}
	}
	return bpeTokens
}

func collapseWhitespace(s string) string {
	return strings.Join(strings.Fields(s), " ")
}

func splitCLIPWords(text string) []string {
	var out []string
	i := 0
	for i < len(text) {
		for i < len(text) && text[i] == ' ' {
			i++
		}
		if i >= len(text) {
			break
		}
		j := i
		for j < len(text) && text[j] != ' ' {
			j++
		}
		out = append(out, text[i:j])
		i = j
	}
	return out
}

func (t *bpeTokenizer) bpe(token string) []string {
	if len(token) == 0 {
		return nil
	}
	runes := []rune(token)
	if len(runes) == 1 {
		return []string{token + "</w>"}
	}
	word := make([]string, len(runes))
	for i, r := range runes[:len(runes)-1] {
		word[i] = string(r)
	}
	word[len(runes)-1] = string(runes[len(runes)-1]) + "</w>"
	pairs := getPairs(word)
	if len(pairs) == 0 {
		return word
	}
	for {
		bestPair := ""
		bestRank := int(^uint(0) >> 1)
		for pair := range pairs {
			rank, ok := t.bpeRanks[pair]
			if !ok {
				continue
			}
			if rank < bestRank {
				bestRank = rank
				bestPair = pair
			}
		}
		if bestPair == "" {
			break
		}
		parts := strings.Split(bestPair, " ")
		first, second := parts[0], parts[1]
		var newWord []string
		i := 0
		for i < len(word) {
			j := indexOf(word, first, i)
			if j == -1 {
				newWord = append(newWord, word[i:]...)
				break
			}
			newWord = append(newWord, word[i:j]...)
			if j < len(word)-1 && word[j] == first && word[j+1] == second {
				newWord = append(newWord, first+second)
				i = j + 2
			} else {
				newWord = append(newWord, word[j])
				i = j + 1
			}
		}
		word = newWord
		if len(word) == 1 {
			break
		}
		pairs = getPairs(word)
	}
	return word
}

func getPairs(word []string) map[string]struct{} {
	out := make(map[string]struct{})
	for i := 0; i < len(word)-1; i++ {
		out[word[i]+" "+word[i+1]] = struct{}{}
	}
	return out
}

func indexOf(slice []string, val string, start int) int {
	for i := start; i < len(slice); i++ {
		if slice[i] == val {
			return i
		}
	}
	return -1
}

// bytesToUnicode maps bytes to unicode runes (GPT-2 / CLIP style).
func bytesToUnicode() map[byte]rune {
	bs := make([]int, 0, 256)
	for b := int('!'); b <= int('~'); b++ {
		bs = append(bs, b)
	}
	for b := int('¡'); b <= int('¬'); b++ {
		bs = append(bs, b)
	}
	for b := int('®'); b <= int('ÿ'); b++ {
		bs = append(bs, b)
	}
	out := make(map[byte]rune, 256)
	n := 0
	for b := 0; b < 256; b++ {
		if containsInt(bs, b) {
			out[byte(b)] = rune(b)
		} else {
			out[byte(b)] = rune(256 + n)
			n++
		}
	}
	return out
}

func containsInt(xs []int, v int) bool {
	for _, x := range xs {
		if x == v {
			return true
		}
	}
	return false
}
