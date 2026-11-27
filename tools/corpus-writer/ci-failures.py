#!/usr/bin/env python3
"""usage: failures.py <run id>  -> failing tests per version (downloads junit artifacts)"""
import sys,subprocess,glob,os,shutil,collections
import xml.etree.ElementTree as ET
run=sys.argv[1]
d=os.path.expanduser(os.environ.get('WORK','~/.cache/corpus-writer'))+'/ci/run-'+run
shutil.rmtree(d,ignore_errors=True); os.makedirs(d)
# Several jobs upload an artifact with the same name (one per app and leg), and `gh run download -p` keeps only
# one of each name. Download every junit artifact by its id into its own folder instead.
R='repos/aldevtools/BusinessCentral.AL.Language.Tests/actions'
arts=subprocess.run(['gh','api','--paginate',f'{R}/runs/{run}/artifacts','--jq','.artifacts[]|select(.name|startswith("junit-test-results-"))|"\\(.id) \\(.name)"'],capture_output=True,text=True).stdout.split('\n')
for a in filter(None,arts):
    aid,name=a.split(' ',1); z=f'{d}/{aid}.zip'
    with open(z,'wb') as fh: subprocess.run(['gh','api',f'{R}/artifacts/{aid}/zip'],stdout=fh)
    shutil.unpack_archive(z,f'{d}/{name}-{aid}','zip'); os.remove(z)
res=collections.defaultdict(dict)
for dd in sorted(glob.glob(d+'/junit-test-results-*')):
    v=dd.split('-')[-2]
    for p in glob.glob(dd+'/*.xml'):
        for tc in ET.parse(p).iter('testcase'):
            f=tc.find('failure')
            if f is None: f=tc.find('error')
            if f is None: continue
            res[(tc.get('classname'),tc.get('name'))][v]=((f.get('message') or '')+' '+(f.text or ''))[:int(sys.argv[2]) if len(sys.argv)>2 else 260].replace('\n',' ')
for k,v in sorted(res.items()):
    print(k,sorted(v)); print('    ',list(v.values())[0])
print(len(res),'failing tests')
