import importlib.util,pathlib,unittest
ROOT=pathlib.Path(__file__).resolve().parents[1]
class BridgePlanTests(unittest.TestCase):
 def load(self):
  path=ROOT/'scripts/deploy/ola_reconcile.py'
  self.assertTrue(path.exists(),'Bridge reconciler is available')
  spec=importlib.util.spec_from_file_location('bridge',path);module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module);return module
 def device(self,plugin,alias,uid,inputs=(),outputs=()):
  return {'plugin_id':plugin,'alias':alias,'id':uid,'input_ports':list(inputs),'output_ports':list(outputs)}
 def test_dynamic_aliases_and_directions(self):
  m=self.load();devices=[self.device(2,71,'artnet',[{'id':3,'universe':None}]),self.device(13,42,'ftdi-serial',outputs=[{'id':8,'universe':None}])]
  plan=m.make_plan(devices)
  self.assertEqual([(p['alias'],p['port'],p['is_output']) for p in plan['patches']],[(71,3,False),(42,8,True)])
  self.assertTrue(plan['usb_found'])
 def test_waits_without_usb_and_ignores_other_plugins(self):
  m=self.load();plan=m.make_plan([self.device(2,8,'art',[{'id':0,'universe':0}]),self.device(5,1,'usbpro',outputs=[{'id':0,'universe':None}])]);self.assertEqual(plan['patches'],[]);self.assertFalse(plan['usb_found'])
 def test_already_patched_is_idempotent(self):
  m=self.load();self.assertEqual(m.make_plan([self.device(2,99,'art',[{'id':0,'universe':0}]),self.device(13,88,'ftdi',outputs=[{'id':0,'universe':0}])])['patches'],[])
 def test_multiple_adapters_require_explicit_uid(self):
  m=self.load();devices=[self.device(2,2,'art',[{'id':0,'universe':None}]),self.device(13,4,'one',outputs=[{'id':0,'universe':None}]),self.device(13,3,'two',outputs=[{'id':0,'universe':None}])]
  with self.assertRaises(ValueError):m.make_plan(devices)
  self.assertEqual(m.make_plan(devices,'two')['ftdi_uid'],'two')
if __name__=='__main__':unittest.main()
