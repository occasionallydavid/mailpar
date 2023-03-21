from setuptools import setup
from setuptools_rust import Binding, RustExtension

setup(
    name="mailpar",
    version="1.0",
    rust_extensions=[
        RustExtension("mailpar", binding=Binding.PyO3)
    ],
    #packages=["mailpar"],
    # rust extensions are not zip safe, just like C-extensions.
    zip_safe=False,
)
