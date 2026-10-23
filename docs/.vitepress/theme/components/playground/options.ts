import type { FormatOptions, TextOptions } from '@gdfmt/wasm'
import type { PlaygroundMessages } from './i18n'

export type PlaygroundOptions = {
  language: Required<NonNullable<TextOptions['language']>>
  format: Required<FormatOptions>
}

export function createDefaultOptions(): PlaygroundOptions {
  return {
    language: { godotVersion: '4.7' },
    format: {
      lineWidth: 100,
      indentStyle: 'tab',
      indentWidth: 4,
      lineEnding: 'lf',
      maxBlankLines: 2,
      finalNewline: true,
      arrayLayout: 'auto',
      dictionaryLayout: 'auto',
      enumLayout: 'auto',
      dictionarySpacing: 'compact',
      trailingCommaLayout: 'auto',
      blankLinesAroundDefinitions: 'preserve',
      lambdaBodyStyle: 'auto',
      inlineCommentSpacing: 1,
    },
  }
}

interface OptionField {
  key: Exclude<keyof FormatOptions, 'finalNewline'>
  values?: readonly (string | number)[]
  min?: number
  max?: number
}

export const optionFields: readonly OptionField[] = [
  { key: 'lineWidth', min: 40, max: 320 },
  {
    key: 'indentStyle',
    values: ['tab', 'space'],
  },
  {
    key: 'indentWidth',
    values: [1, 2, 3, 4, 5, 6, 7, 8],
  },
  {
    key: 'lineEnding',
    values: ['lf', 'crlf', 'preserve'],
  },
  {
    key: 'maxBlankLines',
    values: [0, 1, 2, 3, 4, 5],
  },
  {
    key: 'arrayLayout',
    values: ['auto', 'multiline', 'preserve'],
  },
  {
    key: 'dictionaryLayout',
    values: ['auto', 'multiline', 'preserve'],
  },
  {
    key: 'enumLayout',
    values: ['auto', 'multiline', 'preserve'],
  },
  {
    key: 'dictionarySpacing',
    values: ['compact', 'spaced'],
  },
  {
    key: 'trailingCommaLayout',
    values: ['auto', 'multiline'],
  },
  {
    key: 'blankLinesAroundDefinitions',
    values: ['preserve', 0, 1, 2, 3, 4, 5],
  },
  {
    key: 'lambdaBodyStyle',
    values: ['auto', 'multiline'],
  },
  {
    key: 'inlineCommentSpacing',
    values: [1, 2, 3, 4],
  },
]

interface PlaygroundExample {
  id: keyof PlaygroundMessages['examples']
  source: string
}

export const examples: readonly PlaygroundExample[] = [
  {
    id: 'player',
    source: `extends CharacterBody2D

@export var speed :float=  300.0
@export var jump_velocity:float =-400.0
var inventory= [ "sword",  "shield","health_potion"  ]
var stats = { "health" :100,"mana":  50, "level" :1 }

func _physics_process( delta :float )-> void:
  # Apply gravity while the player is in the air.
  if not is_on_floor( ):
    velocity +=get_gravity( )*delta
  if Input.is_action_just_pressed( "ui_accept" ) and is_on_floor():
    velocity.y =jump_velocity
  var direction :=Input.get_axis("ui_left" ,"ui_right" )
  if direction:
    velocity.x= direction *speed
  else:
    velocity.x =move_toward( velocity.x,0,  speed )
  move_and_slide( )
`,
  },
  {
    id: 'collections',
    source: `extends Node

enum State {IDLE,  RUNNING,JUMPING, }
var items= [ "sword",  "shield","potion", ]
var settings ={ "volume" :0.8,"fullscreen":true, "resolution" :[1920,  1080] }

func _ready( )->void:
  var scores = [10,  25,50,   100 ]
  var doubled=scores.map( func( score ):return score *2 )
  print( doubled )      # Show the new scores.
`,
  },
  {
    id: 'comments',
    source: `extends Node

## A small example with a hand-aligned table.
var score  =0      # Current score.

# gdfmt: formatter-off
var table = [
	[1,  2,  3],
	[10, 20, 30],
]
# gdfmt: formatter-on

func add_score( amount :int )->void:
  score +=amount
  print( "Score: ",score )
`,
  },
]
