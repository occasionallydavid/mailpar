
import mailpar

s = open('/tmp/foo.mbox', 'rb').read()

if 1:
    msg = mailpar.parse_mail(s)
    print('msg:', msg)
    print('msg.path():', msg.path())
    print('msg.raw_byte_offset():', msg.raw_byte_offset())
    print('msg.mime_type():', msg.mime_type())
    print('msg.charset():', msg.charset())
    print('msg.params():', msg.params())
    print('msg.subpart_count():', msg.subpart_count())
    print('msg.get_body():', repr(msg.get_body())[:100])
    print('msg.get_body_raw():', repr(msg.get_body_raw())[:100])
    print()

    for i in range(msg.subpart_count()):
        print(f'msg.subpart({i}):')
        part = msg.subpart(i)
        print('  raw_byte_offset():', part.raw_byte_offset())
        print('  raw_bytes():', repr(part.raw_bytes())[:100])
        print('  raw_body_offset():', part.raw_body_offset())
        print('  raw_body_length():', part.raw_body_length())
        print('  mime_type():', part.mime_type())
        print('  path():', part.path())
        print('  charset():', part.charset())
        print('  params():', msg.params())
        print('  param(boundary):', msg.param("boundary"))
        print('  content_disposition():', part.content_disposition())
        print('  subpart_count():', part.subpart_count())
        print('  get_body():', repr(part.get_body())[:100])
        print('  get_body_raw():', repr(part.get_body_raw())[:100])
        print('  get_body_encoded():', repr(part.get_body_encoded())[:100])
        print()
