
import mailpar

def p(part):
    indent = '  ' * len(part.path())
    pp = lambda *a: print(indent, *a)

    pp('path():', part.path())
    pp('offset():', part.offset())
    pp('raw_bytes():', repr(part.raw_bytes()[:25]))
    pp('body_offset():', part.body_offset())
    pp('mime_type():', part.mime_type())
    pp('path():', part.path())
    pp('charset():', part.charset())
    pp('params():', msg.params())
    pp('param(boundary):', msg.param("boundary"))
    pp('content_disposition():', part.content_disposition())
    pp('subpart_count():', part.subpart_count())
    pp('body():', repr(part.body()[:25]))
    pp('body_encoding():', part.body_encoding())
    pp('body_raw():', repr(part.body_raw()[:25]))
    pp('body_encoded():', repr(part.body_encoded()[:25]))
    pp('headers.offset():', part.headers().offset())
    pp('headers.raw_bytes():', repr(part.headers().raw_bytes()[:20]))
    pp('headers.first(content-type):', part.headers().first('content-type'))
    pp('headers.all(content-type):', part.headers().all('content-type'))
    pp()
    for i in range(part.subpart_count()):
        p(part.subpart(i))


s = open('/tmp/foo.mbox', 'rb').read()
msg = mailpar.from_bytes(s)
p(msg)
