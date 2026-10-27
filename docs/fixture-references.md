# Fixture references and distance

Native Schéma reference behavior was traced in the selected C# snapshot only:

- [DistanceValue_C.Update_](../../scnq-schema/Main/Exports/src/Schema.Blocks.Source.Value.Distance.vl/Schema.Blocks.Source.Value.Distance.vl.1.cs#L82): own/source Position resolution, Auto/Setup and target distance.
- [GetReferencedOrSelectedFixture](../../scnq-schema/Main/Exports/src/Schema.Blocks.Source.Reference.Fixture.vl/Schema.Blocks.Source.Reference.Fixture.vl.1.cs#L348): incoming FixtureReference overrides the Fixture property, then the reference is removed from the returned child state.
- [GetStateOrSetupProperty_Fixture](../../scnq-schema/Main/Exports/src/Schema.PropertyExtensions.vl/Schema.PropertyExtensions.vl.1.cs#L170): reads Setup first, then State using Setup as fallback.
- [GetSelectedFixtureName](../../scnq-schema/Main/Exports/src/Schema.PropertyExtensions.vl/Schema.PropertyExtensions.vl.1.cs#L129): selector comes from the block's Fixture property; this is not the editor's current selection.

## Rust behavior

`Create.Value.Distance` and `Create.Reference.Fixture` retain native identifiers and property names. The inspector renders Fixture as a dropdown of scene fixture names; selecting stores a stable ID. Existing name strings from ScenicScript resolve by name as a fallback. Deleting a target leaves an unresolved reference with a diagnostic, rather than silently retargeting another fixture or computing against the origin. This missing-target policy is an intentional improvement over the export's default-fixture path.

Resolution order:

1. Consume the incoming bank's `fixture_reference` if present; otherwise use the Fixture property.
2. Resolve exact fixture ID first, then the first matching name (as in the native name lookup). IDs are preferred for new edits because names can change.
3. Obtain the target's evaluated Position, falling back to its setup/scene position.
4. For the source: Auto/Setup checked uses setup position; unchecked prefers incoming Position, then the source fixture's evaluated Position, then setup.
5. Write Euclidean distance in metres to Value, preserving other incoming channels and removing the consumed reference.

The target uses evaluated-position fallback in both source modes. Auto/Setup does not force the target into setup mode.

`Spatial.Distance` now supports the same Fixture selector and source-mode flag. To preserve existing point programs, an empty Fixture with no incoming reference uses its Position property as a fixed target. New point and fixture modes are therefore explicit without changing existing saved point programs.

References observe the native live-object update order: a fixture evaluated earlier in the update publishes its new Position; a later fixture still exposes its previous evaluated Position until updated. If it has no Position channel, its current setup position applies immediately. Create.Position.Constant and bank merge/layer flow supply evaluated positions. This matches the inspected source's ordering; gamma execution parity has not been tested.

## Examples

```yaml
name: Distance to a fixture
stack:
  Create.Value.Distance:
    Fixture: fixture-2
    Auto/Setup: false
```

```yaml
name: Reference supplied by a child
stack:
  Create.Value.Distance:
    in:
      - Create.Reference.Fixture:
          Fixture: fixture-2
```

For light falloff, place the distance below Modify.Value.MapFromTo with Source Min 0, Source Max 10, Minimum 1 and Maximum 0, then Convert.Value.ToColor or use the mapped value as a dimming driver. Changes to the target's scene position update the result; reference IDs survive target renaming.
