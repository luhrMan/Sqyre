package gui

import (
	"Dark-And-Darker/structs"
	"fmt"
	"log"
)

type NodeType int

// const (
// 	MacroType NodeType = iota
// 	SequenceType
// 	ActionType
// )

type Node struct {
	Name string
	UID  string
	//Type     NodeType
	Children []*Node
	Parent   *Node
	Action   structs.Action
}

func newActionNode(parent *Node, action structs.Action) *Node {
	actionNum := len(parent.Children) + 1
	uid := fmt.Sprintf("%s.%d", parent.UID, actionNum)
	node := &Node{
		UID:    uid,
		Parent: parent,
		Action: action,
	}

	parent.addChild(node)
	log.Printf("New action: %s %s", uid, action)
	return node
}

func (n *Node) addChild(child *Node) {
	n.Children = append(n.Children, child)
	child.Parent = n
}

func (n *Node) removeChild(child *Node) {
	for i, c := range n.Children {
		if c == child {
			n.Children = append(n.Children[:i], n.Children[i+1:]...)
			log.Printf("Removing %s", child.UID)
			child.Parent = nil
			n.renameChildren()
			return
		}
	}
}

func (n *Node) renameChildren() {
	for i, child := range n.Children {
		child.UID = fmt.Sprintf("%s.%d", n.UID, i+1)
		child.renameChildren()
	}
}
