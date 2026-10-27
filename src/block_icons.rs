pub fn get(name: &str) -> Option<&'static str> {
    match name {
        "plugin-component.svg" => Some(include_str!("../web/icons/plugin-component.svg")),
        "Create.Value.Constant.svg" => Some(include_str!("../web/icons/Create.Value.Constant.svg")),
        "Create.Value.LFO.svg" => Some(include_str!("../web/icons/Create.Value.LFO.svg")),
        "Create.Color.Constant.svg" => Some(include_str!("../web/icons/Create.Color.Constant.svg")),
        "Create.Color.Color.svg" => Some(include_str!("../web/icons/Create.Color.Color.svg")),
        "Create.Layer.Above.svg" => Some(include_str!("../web/icons/Create.Layer.Above.svg")),
        "Modify.Value.Invert.svg" => Some(include_str!("../web/icons/Modify.Value.Invert.svg")),
        "Modify.Value.Clamp.svg" => Some(include_str!("../web/icons/Modify.Value.Clamp.svg")),
        "Modify.Value.MapTo.svg" => Some(include_str!("../web/icons/Modify.Value.MapTo.svg")),
        "Modify.Value.MapFromTo.svg" => {
            Some(include_str!("../web/icons/Modify.Value.MapFromTo.svg"))
        }
        "Modify.Value.Steps.svg" => Some(include_str!("../web/icons/Modify.Value.Steps.svg")),
        "Modify.Value.Integrate.svg" => {
            Some(include_str!("../web/icons/Modify.Value.Integrate.svg"))
        }
        "Modify.SampleAndHold.svg" => Some(include_str!("../web/icons/Modify.SampleAndHold.svg")),
        "Combine.Add.svg" => Some(include_str!("../web/icons/Combine.Add.svg")),
        "Combine.Multiply.svg" => Some(include_str!("../web/icons/Combine.Multiply.svg")),
        "Combine.Crossfade.svg" => Some(include_str!("../web/icons/Combine.Crossfade.svg")),
        "Filter.Manual.svg" => Some(include_str!("../web/icons/Filter.Manual.svg")),
        "Convert.Value.ToColor.svg" => Some(include_str!("../web/icons/Convert.Value.ToColor.svg")),
        "Tool.Merge.svg" => Some(include_str!("../web/icons/Tool.Merge.svg")),
        "Modify.Color.Invert.svg" => Some(include_str!("../web/icons/Modify.Color.Invert.svg")),
        "Modify.Color.Dim.svg" => Some(include_str!("../web/icons/Modify.Color.Dim.svg")),
        "Create.Position.Constant.svg" => {
            Some(include_str!("../web/icons/Create.Position.Constant.svg"))
        }
        "Create.Reference.Fixture.svg" => {
            Some(include_str!("../web/icons/Create.Reference.Fixture.svg"))
        }
        "Create.Value.Distance.svg" => Some(include_str!("../web/icons/Create.Value.Distance.svg")),
        "Spatial.Distance.svg" => Some(include_str!("../web/icons/Spatial.Distance.svg")),
        "Spatial.Axis.svg" => Some(include_str!("../web/icons/Spatial.Axis.svg")),
        "Script.Value.svg" => Some(include_str!("../web/icons/Script.Value.svg")),
        "Light.Emitter.svg" => Some(include_str!("../web/icons/Light.Emitter.svg")),
        "Light.Receiver.svg" => Some(include_str!("../web/icons/Light.Receiver.svg")),
        "Create.Position.Current.svg" => {
            Some(include_str!("../web/icons/Create.Position.Current.svg"))
        }
        "Create.Position.XYZ.svg" => Some(include_str!("../web/icons/Create.Position.XYZ.svg")),
        "Create.Position.Group.Center.svg" => Some(include_str!(
            "../web/icons/Create.Position.Group.Center.svg"
        )),
        "Create.Position.Group.XYZ.svg" => {
            Some(include_str!("../web/icons/Create.Position.Group.XYZ.svg"))
        }
        "Create.Position.Group.Slide.svg" => {
            Some(include_str!("../web/icons/Create.Position.Group.Slide.svg"))
        }
        "Create.Position.Group.Random.svg" => Some(include_str!(
            "../web/icons/Create.Position.Group.Random.svg"
        )),
        "Create.Value.Random.svg" => Some(include_str!("../web/icons/Create.Value.Random.svg")),
        "Modify.Value.RandomizeTrigger.svg" => Some(include_str!(
            "../web/icons/Modify.Value.RandomizeTrigger.svg"
        )),
        "Create.Position.Random.svg" => {
            Some(include_str!("../web/icons/Create.Position.Random.svg"))
        }
        "Create.Position.Bezier.svg" => {
            Some(include_str!("../web/icons/Create.Position.Bezier.svg"))
        }
        "Create.Value.Bezier.svg" => Some(include_str!("../web/icons/Create.Value.Bezier.svg")),
        "Assign.Content.svg" => Some(include_str!("../web/icons/Assign.Content.svg")),
        "Assign.Director.svg" => Some(include_str!("../web/icons/Assign.Director.svg")),
        "Assign.Fixture.svg" => Some(include_str!("../web/icons/Assign.Fixture.svg")),
        "Assign.Global.Color.svg" => Some(include_str!("../web/icons/Assign.Global.Color.svg")),
        "Assign.Group.svg" => Some(include_str!("../web/icons/Assign.Group.svg")),
        "Combine.Audio.Add.svg" => Some(include_str!("../web/icons/Combine.Audio.Add.svg")),
        "Combine.Audio.Multiply.svg" => {
            Some(include_str!("../web/icons/Combine.Audio.Multiply.svg"))
        }
        "Combine.Color.Distance.svg" => {
            Some(include_str!("../web/icons/Combine.Color.Distance.svg"))
        }
        "Combine.Color.Max.svg" => Some(include_str!("../web/icons/Combine.Color.Max.svg")),
        "Combine.Color.Mix.svg" => Some(include_str!("../web/icons/Combine.Color.Mix.svg")),
        "Combine.Texture.Multiply.svg" => {
            Some(include_str!("../web/icons/Combine.Texture.Multiply.svg"))
        }
        "Combine.Value.Boolean.svg" => Some(include_str!("../web/icons/Combine.Value.Boolean.svg")),
        "Combine.Value.Compare.svg" => Some(include_str!("../web/icons/Combine.Value.Compare.svg")),
        "Convert.Audio.Meter.svg" => Some(include_str!("../web/icons/Convert.Audio.Meter.svg")),
        "Convert.Speed.PositionToTarget.svg" => Some(include_str!(
            "../web/icons/Convert.Speed.PositionToTarget.svg"
        )),
        "Convert.Value.ToDuration.svg" => {
            Some(include_str!("../web/icons/Convert.Value.ToDuration.svg"))
        }
        "Convert.Value.ToSpeed.svg" => Some(include_str!("../web/icons/Convert.Value.ToSpeed.svg")),
        "Create.AnimationWheel.Position.svg" => Some(include_str!(
            "../web/icons/Create.AnimationWheel.Position.svg"
        )),
        "Create.AnimationWheel.Rotation.svg" => Some(include_str!(
            "../web/icons/Create.AnimationWheel.Rotation.svg"
        )),
        "Create.Audio.Input.svg" => Some(include_str!("../web/icons/Create.Audio.Input.svg")),
        "Create.Audio.OSC.svg" => Some(include_str!("../web/icons/Create.Audio.OSC.svg")),
        "Create.Audio.Sample.svg" => Some(include_str!("../web/icons/Create.Audio.Sample.svg")),
        "Create.BeamShaper.Position.svg" => {
            Some(include_str!("../web/icons/Create.BeamShaper.Position.svg"))
        }
        "Create.Blade.Position.svg" => Some(include_str!("../web/icons/Create.Blade.Position.svg")),
        "Create.Blade.Rotation.svg" => Some(include_str!("../web/icons/Create.Blade.Rotation.svg")),
        "Create.Color.Audio.FFTPixels.svg" => Some(include_str!(
            "../web/icons/Create.Color.Audio.FFTPixels.svg"
        )),
        "Create.Color.Audio.Level.svg" => {
            Some(include_str!("../web/icons/Create.Color.Audio.Level.svg"))
        }
        "Create.Color.External.svg" => Some(include_str!("../web/icons/Create.Color.External.svg")),
        "Create.Color.Flash.Beat.svg" => {
            Some(include_str!("../web/icons/Create.Color.Flash.Beat.svg"))
        }
        "Create.Color.Flash.BPM.svg" => {
            Some(include_str!("../web/icons/Create.Color.Flash.BPM.svg"))
        }
        "Create.Color.Flash.Timed.svg" => {
            Some(include_str!("../web/icons/Create.Color.Flash.Timed.svg"))
        }
        "Create.Color.Global.svg" => Some(include_str!("../web/icons/Create.Color.Global.svg")),
        "Create.Color.OSC.svg" => Some(include_str!("../web/icons/Create.Color.OSC.svg")),
        "Create.Color.Pixel.Distance.Axis.svg" => Some(include_str!(
            "../web/icons/Create.Color.Pixel.Distance.Axis.svg"
        )),
        "Create.Color.Pixel.Distance.Plane.svg" => Some(include_str!(
            "../web/icons/Create.Color.Pixel.Distance.Plane.svg"
        )),
        "Create.Color.Pixel.Distance.Point.svg" => Some(include_str!(
            "../web/icons/Create.Color.Pixel.Distance.Point.svg"
        )),
        "Create.Color.Pixel.Pulse.Cannon.svg" => Some(include_str!(
            "../web/icons/Create.Color.Pixel.Pulse.Cannon.svg"
        )),
        "Create.Color.Pixel.Pulse.Manual.svg" => Some(include_str!(
            "../web/icons/Create.Color.Pixel.Pulse.Manual.svg"
        )),
        "Create.Color.Pixel.Pulse.svg" => {
            Some(include_str!("../web/icons/Create.Color.Pixel.Pulse.svg"))
        }
        "Create.Color.Pixel.Rainbow.svg" => {
            Some(include_str!("../web/icons/Create.Color.Pixel.Rainbow.svg"))
        }
        "Create.Color.Pixel.Segment.svg" => {
            Some(include_str!("../web/icons/Create.Color.Pixel.Segment.svg"))
        }
        "Create.Color.Primary.svg" => Some(include_str!("../web/icons/Create.Color.Primary.svg")),
        "Create.Color.Universe.svg" => Some(include_str!("../web/icons/Create.Color.Universe.svg")),
        "Create.Copycat.svg" => Some(include_str!("../web/icons/Create.Copycat.svg")),
        "Create.Duration.Constant.svg" => {
            Some(include_str!("../web/icons/Create.Duration.Constant.svg"))
        }
        "Create.Focus.Value.svg" => Some(include_str!("../web/icons/Create.Focus.Value.svg")),
        "Create.Frost.Value.svg" => Some(include_str!("../web/icons/Create.Frost.Value.svg")),
        "Create.Gobo.Constant.svg" => Some(include_str!("../web/icons/Create.Gobo.Constant.svg")),
        "Create.Gobo.Rotate.Value.svg" => {
            Some(include_str!("../web/icons/Create.Gobo.Rotate.Value.svg"))
        }
        "Create.Gobo.Shake.Constant.svg" => {
            Some(include_str!("../web/icons/Create.Gobo.Shake.Constant.svg"))
        }
        "Create.Iris.Value.svg" => Some(include_str!("../web/icons/Create.Iris.Value.svg")),
        "Create.Macro.Constant.svg" => Some(include_str!("../web/icons/Create.Macro.Constant.svg")),
        "Create.Prism.Rotate.svg" => Some(include_str!("../web/icons/Create.Prism.Rotate.svg")),
        "Create.Prism.Select.svg" => Some(include_str!("../web/icons/Create.Prism.Select.svg")),
        "Create.Radius.Constant.svg" => {
            Some(include_str!("../web/icons/Create.Radius.Constant.svg"))
        }
        "Create.Reference.SequenceOffset.svg" => Some(include_str!(
            "../web/icons/Create.Reference.SequenceOffset.svg"
        )),
        "Create.Remote.svg" => Some(include_str!("../web/icons/Create.Remote.svg")),
        "Create.Resource.Constant.svg" => {
            Some(include_str!("../web/icons/Create.Resource.Constant.svg"))
        }
        "Create.Rotation.Audio.Shake.svg" => {
            Some(include_str!("../web/icons/Create.Rotation.Audio.Shake.svg"))
        }
        "Create.Rotation.Constant.svg" => {
            Some(include_str!("../web/icons/Create.Rotation.Constant.svg"))
        }
        "Create.Rotation.Follow.svg" => {
            Some(include_str!("../web/icons/Create.Rotation.Follow.svg"))
        }
        "Create.Rotation.PanTilt.svg" => {
            Some(include_str!("../web/icons/Create.Rotation.PanTilt.svg"))
        }
        "Create.Rotation.Perpendicular.svg" => Some(include_str!(
            "../web/icons/Create.Rotation.Perpendicular.svg"
        )),
        "Create.Rotation.Timed.svg" => Some(include_str!("../web/icons/Create.Rotation.Timed.svg")),
        "Create.Rotation.XYZ.svg" => Some(include_str!("../web/icons/Create.Rotation.XYZ.svg")),
        "Create.Shapes.ResourceMode.Constant.svg" => Some(include_str!(
            "../web/icons/Create.Shapes.ResourceMode.Constant.svg"
        )),
        "Create.Speed.Constant.svg" => Some(include_str!("../web/icons/Create.Speed.Constant.svg")),
        "Create.Temperature.Constant.svg" => {
            Some(include_str!("../web/icons/Create.Temperature.Constant.svg"))
        }
        "Create.Texture.Color.svg" => Some(include_str!("../web/icons/Create.Texture.Color.svg")),
        "Create.Texture.Noise.svg" => Some(include_str!("../web/icons/Create.Texture.Noise.svg")),
        "Create.Value.Audio.Beat.svg" => {
            Some(include_str!("../web/icons/Create.Value.Audio.Beat.svg"))
        }
        "Create.Value.Audio.FFT16.svg" => {
            Some(include_str!("../web/icons/Create.Value.Audio.FFT16.svg"))
        }
        "Create.Value.Audio.Level.svg" => {
            Some(include_str!("../web/icons/Create.Value.Audio.Level.svg"))
        }
        "Create.Value.Audio.PeakPower.svg" => Some(include_str!(
            "../web/icons/Create.Value.Audio.PeakPower.svg"
        )),
        "Create.Value.Audio.Pitch.svg" => {
            Some(include_str!("../web/icons/Create.Value.Audio.Pitch.svg"))
        }
        "Create.Value.Audio.Power.svg" => {
            Some(include_str!("../web/icons/Create.Value.Audio.Power.svg"))
        }
        "Create.Value.Closeness.svg" => {
            Some(include_str!("../web/icons/Create.Value.Closeness.svg"))
        }
        "Create.Value.Control.svg" => Some(include_str!("../web/icons/Create.Value.Control.svg")),
        "Create.Value.Controller.Pad.svg" => {
            Some(include_str!("../web/icons/Create.Value.Controller.Pad.svg"))
        }
        "Create.Value.Controller.Value.svg" => Some(include_str!(
            "../web/icons/Create.Value.Controller.Value.svg"
        )),
        "Create.Value.Expression.svg" => {
            Some(include_str!("../web/icons/Create.Value.Expression.svg"))
        }
        "Create.Value.Fixture.IsSelected.svg" => Some(include_str!(
            "../web/icons/Create.Value.Fixture.IsSelected.svg"
        )),
        "Create.Value.Global.svg" => Some(include_str!("../web/icons/Create.Value.Global.svg")),
        "Create.Value.Group.AxisPosition.svg" => Some(include_str!(
            "../web/icons/Create.Value.Group.AxisPosition.svg"
        )),
        "Create.Value.Keyboard.svg" => Some(include_str!("../web/icons/Create.Value.Keyboard.svg")),
        "Create.Value.OSC.svg" => Some(include_str!("../web/icons/Create.Value.OSC.svg")),
        "Create.Value.OSCReceived.svg" => {
            Some(include_str!("../web/icons/Create.Value.OSCReceived.svg"))
        }
        "Create.Value.SequenceIndex.svg" => {
            Some(include_str!("../web/icons/Create.Value.SequenceIndex.svg"))
        }
        "Create.Value.System.FileExists.svg" => Some(include_str!(
            "../web/icons/Create.Value.System.FileExists.svg"
        )),
        "Create.Value.Trigger.svg" => Some(include_str!("../web/icons/Create.Value.Trigger.svg")),
        "Create.Value.XBox.svg" => Some(include_str!("../web/icons/Create.Value.XBox.svg")),
        "Create.Zoom.Value.svg" => Some(include_str!("../web/icons/Create.Zoom.Value.svg")),
        "Filter.Audio.Power.svg" => Some(include_str!("../web/icons/Filter.Audio.Power.svg")),
        "Global.Set.Color.svg" => Some(include_str!("../web/icons/Global.Set.Color.svg")),
        "Global.Set.Primary.FromColor.svg" => Some(include_str!(
            "../web/icons/Global.Set.Primary.FromColor.svg"
        )),
        "Modify.Color.Alpha.AsAlpha.svg" => {
            Some(include_str!("../web/icons/Modify.Color.Alpha.AsAlpha.svg"))
        }
        "Modify.Color.AttackDecay.svg" => {
            Some(include_str!("../web/icons/Modify.Color.AttackDecay.svg"))
        }
        "Modify.Color.Blur.svg" => Some(include_str!("../web/icons/Modify.Color.Blur.svg")),
        "Modify.Color.Clamp.svg" => Some(include_str!("../web/icons/Modify.Color.Clamp.svg")),
        "Modify.Color.Complement.svg" => {
            Some(include_str!("../web/icons/Modify.Color.Complement.svg"))
        }
        "Modify.Color.EvenOdd.svg" => Some(include_str!("../web/icons/Modify.Color.EvenOdd.svg")),
        "Modify.Color.Flickerize.svg" => {
            Some(include_str!("../web/icons/Modify.Color.Flickerize.svg"))
        }
        "Modify.Color.Hue.Rotate.svg" => {
            Some(include_str!("../web/icons/Modify.Color.Hue.Rotate.svg"))
        }
        "Modify.Color.Paint.AlphaBased.svg" => Some(include_str!(
            "../web/icons/Modify.Color.Paint.AlphaBased.svg"
        )),
        "Modify.Color.Paint.BlackAndWhite.svg" => Some(include_str!(
            "../web/icons/Modify.Color.Paint.BlackAndWhite.svg"
        )),
        "Modify.Color.Rainbow.svg" => Some(include_str!("../web/icons/Modify.Color.Rainbow.svg")),
        "Modify.Color.RangeSlice.svg" => {
            Some(include_str!("../web/icons/Modify.Color.RangeSlice.svg"))
        }
        "Modify.Color.Saturation.Set.svg" => {
            Some(include_str!("../web/icons/Modify.Color.Saturation.Set.svg"))
        }
        "Modify.Color.Scramble.svg" => Some(include_str!("../web/icons/Modify.Color.Scramble.svg")),
        "Modify.Color.Shift.svg" => Some(include_str!("../web/icons/Modify.Color.Shift.svg")),
        "Modify.Color.Strobe.svg" => Some(include_str!("../web/icons/Modify.Color.Strobe.svg")),
        "Modify.ColorWheel.Offset.svg" => {
            Some(include_str!("../web/icons/Modify.ColorWheel.Offset.svg"))
        }
        "Modify.Delay.svg" => Some(include_str!("../web/icons/Modify.Delay.svg")),
        "Modify.Deprecated.Color.Flickerize.svg" => Some(include_str!(
            "../web/icons/Modify.Deprecated.Color.Flickerize.svg"
        )),
        "Modify.Memorize.svg" => Some(include_str!("../web/icons/Modify.Memorize.svg")),
        "Modify.Repeat.svg" => Some(include_str!("../web/icons/Modify.Repeat.svg")),
        "Modify.Value.ADSR.svg" => Some(include_str!("../web/icons/Modify.Value.ADSR.svg")),
        "Modify.Value.AttackDecay.svg" => {
            Some(include_str!("../web/icons/Modify.Value.AttackDecay.svg"))
        }
        "Modify.Value.Cooldown.svg" => Some(include_str!("../web/icons/Modify.Value.Cooldown.svg")),
        "Modify.Value.Damper.svg" => Some(include_str!("../web/icons/Modify.Value.Damper.svg")),
        "Modify.Value.Edge.Falling.svg" => {
            Some(include_str!("../web/icons/Modify.Value.Edge.Falling.svg"))
        }
        "Modify.Value.Edge.Rising.svg" => {
            Some(include_str!("../web/icons/Modify.Value.Edge.Rising.svg"))
        }
        "Modify.Value.Hold.svg" => Some(include_str!("../web/icons/Modify.Value.Hold.svg")),
        "Modify.Value.Map.HalfUpDown.svg" => {
            Some(include_str!("../web/icons/Modify.Value.Map.HalfUpDown.svg"))
        }
        "Modify.Value.RunningMap.svg" => {
            Some(include_str!("../web/icons/Modify.Value.RunningMap.svg"))
        }
        "Modify.Value.SelectRange.svg" => {
            Some(include_str!("../web/icons/Modify.Value.SelectRange.svg"))
        }
        "Modify.Value.Shape.svg" => Some(include_str!("../web/icons/Modify.Value.Shape.svg")),
        "Modify.Value.Smooth.svg" => Some(include_str!("../web/icons/Modify.Value.Smooth.svg")),
        "Modify.Value.SmoothAcceleration.svg" => Some(include_str!(
            "../web/icons/Modify.Value.SmoothAcceleration.svg"
        )),
        "Modify.Value.Toggle.svg" => Some(include_str!("../web/icons/Modify.Value.Toggle.svg")),
        "Modify.Value.Wave.svg" => Some(include_str!("../web/icons/Modify.Value.Wave.svg")),
        "Pick.Alternate.svg" => Some(include_str!("../web/icons/Pick.Alternate.svg")),
        "Pick.Audio.Beat.svg" => Some(include_str!("../web/icons/Pick.Audio.Beat.svg")),
        "Pick.Audio.Power.svg" => Some(include_str!("../web/icons/Pick.Audio.Power.svg")),
        "Pick.BPM.svg" => Some(include_str!("../web/icons/Pick.BPM.svg")),
        "Pick.Channel.svg" => Some(include_str!("../web/icons/Pick.Channel.svg")),
        "Pick.Controller.svg" => Some(include_str!("../web/icons/Pick.Controller.svg")),
        "Pick.Group.svg" => Some(include_str!("../web/icons/Pick.Group.svg")),
        "Pick.LTP.svg" => Some(include_str!("../web/icons/Pick.LTP.svg")),
        "Pick.Time.svg" => Some(include_str!("../web/icons/Pick.Time.svg")),
        "S.Pong.Paddle.svg" => Some(include_str!("../web/icons/S.Pong.Paddle.svg")),
        "S.Pong.Position.svg" => Some(include_str!("../web/icons/S.Pong.Position.svg")),
        "Setup.Pixels.Circle.svg" => Some(include_str!("../web/icons/Setup.Pixels.Circle.svg")),
        "Test.Widgets.FilePicker.svg" => {
            Some(include_str!("../web/icons/Test.Widgets.FilePicker.svg"))
        }
        "Tool.Deprecated.Index.Counter.svg" => Some(include_str!(
            "../web/icons/Tool.Deprecated.Index.Counter.svg"
        )),
        "Tool.Deprecated.Pick.Index.svg" => {
            Some(include_str!("../web/icons/Tool.Deprecated.Pick.Index.svg"))
        }
        "Transition.Fade.svg" => Some(include_str!("../web/icons/Transition.Fade.svg")),
        _ => None,
    }
}
