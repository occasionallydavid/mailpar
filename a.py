
import mailpar

print((mailpar.parse_mail(b"x"*100000)))
print((mailpar.parse_mail(open('/tmp/foo2.eml', 'rb').read())))
