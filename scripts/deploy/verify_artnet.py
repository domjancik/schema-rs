#!/usr/bin/python3
"""Isolated local-engine -> OLA test. Refuses nonzero testing when FTDI output exists."""
import json,os,pathlib,signal,subprocess,tempfile,time,urllib.request
from ola.ClientWrapper import ClientWrapper

def call(method,*args):
    wrapper=ClientWrapper();result=[];errors=[]
    def done(status,*values):
        if status.Succeeded():result.extend(values)
        else:errors.append(str(status))
        wrapper.Stop()
    getattr(wrapper.Client(),method)(*args,done) if method=='FetchDmx' else getattr(wrapper.Client(),method)(done)
    wrapper.AddEvent(5000,wrapper.Stop);wrapper.Run()
    if errors or not result:raise RuntimeError('OLA RPC failed or timed out: '+';'.join(errors))
    return result

def main():
    devices=call('FetchDevices')[0]
    if any(d.plugin_id==13 and d.output_ports for d in devices):raise SystemExit('FTDI hardware is connected; refusing nonzero software-only test')
    token=json.loads(pathlib.Path('/etc/schema/access.json').read_text())['schema_token']
    request=urllib.request.Request('http://127.0.0.1:8787/api/state',headers={'x-schema-token':token})
    p=json.load(urllib.request.urlopen(request))['project']
    assert len(p['fixtures'])==1 and p['fixtures'][0]['dmx_model']=='uv-cob'
    assert not p['output']['enabled']
    fixture=p['fixtures'][0]['id'];node=p['content'][fixture][0]['root'];assert node['behavior']=='Create.Value.Constant' and node['properties']['Value']==0
    with tempfile.TemporaryDirectory(prefix='schema-artnet-check-') as directory:
        root=pathlib.Path(directory)
        for folder in ['registry/layouts','registry/content','registry/config']:(root/folder).mkdir(parents=True)
        (root/'registry/layouts/init.json').write_text(json.dumps({k:p[k] for k in ['version','name','fixtures','dmx_models','dimming']}))
        (root/'registry/content/init.json').write_text(json.dumps([{'fixture':fixture,'layer':i,'stack':stack} for i,stack in enumerate(p['content'][fixture])]))
        (root/'registry/config/output.json').write_text(json.dumps(p['output']))
        env=dict(os.environ);env.pop('SCHEMA_TOKEN',None)
        process=subprocess.Popen(['/opt/schema/current/schema-rs','serve','--project',directory,'--bind','127.0.0.1:8819','--artnet','127.0.0.1:6454'],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE,env=env)
        def api(path,body=None):
            r=urllib.request.Request('http://127.0.0.1:8819'+path,data=json.dumps(body).encode() if body is not None else None,headers={'content-type':'application/json'})
            return json.load(urllib.request.urlopen(r,timeout=5))
        def value(amount):
            state=api('/api/state');api('/api/command',{'expected_revision':state['snapshot']['revision'],'command':{'type':'set_property','fixture':fixture,'layer':0,'node':node['id'],'key':'Value','value':amount}})
        def wait_bytes(first):
            for attempt in range(60):
                _,data=call('FetchDmx',0);data=list(data)
                if len(data)==512 and data[0]==first and all(v==0 for v in data[1:]):return data
                time.sleep(.05)
            raise AssertionError('Expected UV dimmer/program packet did not reach OLA')
        try:
            for attempt in range(50):
                try:api('/api/state');break
                except OSError:time.sleep(.1)
            else:raise RuntimeError('Scratch engine did not start')
            value(.5);data=wait_bytes(128);print('ARM engine -> Art-Net -> OLA universe 0 verified: dimmer 128, program 0, remaining slots zero')
            value(0);wait_bytes(0)
        finally:
            try:value(0);wait_bytes(0)
            finally:
                process.send_signal(signal.SIGINT);process.wait(timeout=20)
        _,last=call('FetchDmx',0);assert all(v==0 for v in last)
        print('Scratch sender stopped; OLA universe 0 remains all zero; production scene/history unchanged')
if __name__=='__main__':main()
