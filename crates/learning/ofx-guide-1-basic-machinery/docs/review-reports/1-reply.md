1. The plugin identifier and icon filename are not mismatched. At least DaVinci
   Resolve recognizes the icon correctly without any issues.
2. Considering that the lack of a `CFBundleExecutable` key in the `Info.plist`
   has yet to cause problems, let's leave it as-is for now, since I'm uncertain
   what is the correct value to use there.
3. `match true` is intentional, so that the first predicate can be visually
   aligned with the rest.
