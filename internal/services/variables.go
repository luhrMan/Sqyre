package services

import "Sqyre/internal/macro"

type EntryValidation = macro.EntryValidation

var (
	ResolveVariables            = macro.ResolveVariables
	ParseVariableReference      = macro.ParseVariableReference
	EvaluateCondition           = macro.EvaluateCondition
	UnknownVariableWarning      = macro.UnknownVariableWarning
	LooksLikeExpression         = macro.LooksLikeExpression
	ValidateVariableReferences  = macro.ValidateVariableReferences
	ValidateNumericExpression   = macro.ValidateNumericExpression
	ValidateCalculateExpression = macro.ValidateCalculateExpression
	ValidateSetVariableValue    = macro.ValidateSetVariableValue
	PreviewCalculate            = macro.PreviewCalculate
	EvaluateExpression          = macro.EvaluateExpression
	ResolveInt                  = macro.ResolveInt
	ResolveFloat                = macro.ResolveFloat
	ResolveSetVariableValue     = macro.ResolveSetVariableValue
	ResolveString               = macro.ResolveString
	ResolveSearchAreaCoords     = macro.ResolveSearchAreaCoords
	ResolveSearchAreaCoordsFromRef = macro.ResolveSearchAreaCoordsFromRef
	ResolveIntWithOverrides        = macro.ResolveIntWithOverrides
	LookupPoint                 = macro.LookupPoint
	DefaultResolutionKey        = macro.DefaultResolutionKey
	LogPanicToFile              = macro.LogPanicToFile
	GoSafe                      = macro.GoSafe
	ParseVarRefSegments         = macro.ParseVarRefSegments
	TextContainsVarRef          = macro.TextContainsVarRef
)

var OnPanicNotifyUser = macro.OnPanicNotifyUser

type SyncWriter = macro.SyncWriter
