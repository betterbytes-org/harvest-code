# Harvest Cross-Language Data Structure Spec (WIP)


Core idea: just embed everything into theory of uninterpreted functions so that we can talk about interface semantics soundly in a cross-language way.



Core uninterpreted functions:
`get: Self -> K -> V`
`set: Self -> K -> V -> Self`

For linear sequence data types:
`get: Self -> int -> V` (specialization of general get)
`set: Self -> int -> V -> Self` (specialization of general set)
`front: Self -> int`
`back: Self -> int` 


shorthand: `old` = version of self before func is ran

# Derived AbstractOps

## Len:: Self -> uint
`self.front() - self.back()`

## PeekBack:: Self -> V
`self.get(self.back() + 1)`

## PeekFront:: Self -> V
`self.get(self.front() - 1)`


## PushFront
`self.set(old.front(), v) &&`  
`self.front() == old.front() - 1`

 
## PushBack:: mut Self -> V  
`self.set(old.back(), v) &&`  
`self.back() == old.back() + 1`

## PopFront
`self.back() == old.back() - 1 &&`
`return old.peek_back()`

## PopBack:: mut Self -> V
`self.back() == old.back() - 1 &&`
`return old.peek_back()`
