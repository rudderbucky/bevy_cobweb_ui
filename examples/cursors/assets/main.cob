#commands
SetPrimaryCursor(Custom{image:"cursor.png" hotspot:(9, 9)})

#scenes
"scene"
    FlexStyle{dims:{width:100vw height:100vh} content:{justify_main:Center justify_cross:Center}}

    "box"
        FlexStyle{dims:{width:200px height:200px} content:{justify_main:Center justify_cross:Center}}
        BackgroundColor(#00BB00)
        Cursor{hover:System(Move)}

        "inner"
            FlexStyle{dims:{width:100px height:100px}}
            BackgroundColor(#0000FF)
            Cursor{hover:System(Grab) press:System(Grabbing)}
            FocusPolicy::Block // This prevents interactions from reaching the lower node and causing cursor race conditions.