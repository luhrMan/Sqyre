package gui

import (
    "Dark-And-Darker/structs"
    "fmt"
	"log"
)

type NodeType int

const (
	MacroType NodeType = iota
	SequenceType
	ActionType
)

type Node struct {
	Name     string
	UID      string
	Type     NodeType
	Children []*Node
	Parent   *Node
	Action   structs.Action
}

func NewMacro(uid string) *Node {
	return &Node{UID: uid, Type: MacroType}
}

func NewSequence(parent *Node, name string) *Node {
	seqNum := len(parent.Children)
	uid := fmt.Sprintf("Seq%d", seqNum+1)
	node := &Node{Name: name, UID: uid, Type: SequenceType, Parent: parent}
	
	parent.AddChild(node)
	log.Printf("New sequence %s: %s", node.UID, node.Name)
	return node
}

func NewAction(parent *Node, action structs.Action) *Node {
	seqNum := getSequenceNumber(parent)
	actionNum := len(parent.Children) + 1
	uid := fmt.Sprintf("Seq%d.%d", seqNum, actionNum)
	node := &Node{UID: uid, Type: ActionType, Parent: parent, Action: action}

	parent.AddChild(node)
	log.Printf("New action: %s %s ", uid, action)
	return node
}

func getSequenceNumber(node *Node) int {
	if node.Type == SequenceType {
		for i, child := range node.Parent.Children {
			if child == node {
				return i + 1
			}
		}
	}
	return 0
}

func (n *Node) RenameSiblings() {
	switch n.Type {
	case 0:
		for a, seq := range n.Children {
			uid := fmt.Sprintf("Seq%d", a+1)
			seq.UID = uid
			for b, action := range seq.Children {
				uid := fmt.Sprintf("Seq%d.%d", a+1, b+1)
				action.UID = uid
			}
		}
	case 1:
		for a, action := range n.Children {
			seqNum := n.UID
			uid := fmt.Sprintf("%s.%d", seqNum, a+1)
			action.UID = uid
		}
	}
}

func (n *Node) AddChild(child *Node) {
	n.Children = append(n.Children, child)
	child.Parent = n
}

func (n *Node) RemoveChild(child *Node) {
	for i, c := range n.Children {
		if c == child {
			n.Children = append(n.Children[:i], n.Children[i+1:]...)
			log.Printf("Removing %s", child.UID)
			child.Parent = nil
			n.RenameSiblings()
			return
		}
	}
}
