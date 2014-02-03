import gdb
import weenix

_char_type = gdb.lookup_type("char")
_list_type = gdb.lookup_type("list_t")
_list_link_type = gdb.lookup_type("list_link_t")

class Link:

	def __init__(self, value, dtype=None, dmemb=None):
		self._value = value
		self._dtype = dtype
		self._dmemb = dmemb

	def value(self):
		return self._value

	def item(self, typ=None, memb=None):
		if (typ == None):
			typ = self._dtype
		if (memb == None):
			memb = self._dmemb
		if (typ == None or memb == None):
			raise RuntimeError("list reference requires "
							   "both type and member name")

		for field in gdb.lookup_type(typ).fields():
			if (field.name == memb):
				link = self._value.address.cast(_char_type.pointer())
				link -= (field.bitpos / 8)
				link = link.cast(gdb.lookup_type(typ).pointer())
				return link.dereference()
		raise weenix.WeenixError("no member {0} of {1}"
								 .format(memb, typ))

	def link_addr(self):
		return self._value.address

class List:

	def __init__(self, value, dtype=None, dmemb=None):
		self._value = value
		self._dtype = dtype
		self._dmemb = dmemb

	def __iter__(self):
		curr = self._value["l_next"].dereference()
		while (curr.address != self._value.address):
			yield Link(curr, self._dtype, self._dmemb)
			curr = curr["l_next"].dereference()
		raise StopIteration

	def __len__(self):
		try:
			return self.__count
		except AttributeError:
			self.__count = 0
			curr = self._value["l_next"].dereference()
			while (curr.address != self._value.address):
				curr = curr["l_next"].dereference()
				self.__count += 1
			return self.__count

	def __getitem__(self, key):
		if (type(key) != int):
			raise TypeError(key)

		for i, item in enumerate(self):
			if (i == key):
				return item
		raise IndexError(key)

def load(name, dtype=None, dmemb=None):
	weenix.assert_type(name, _list_type)

	if (dtype != None):
		try:
			if (not isinstance(dtype, gdb.Type)):
				typ = gdb.lookup_type(dtype)
			else:
				typ = dtype
		except RuntimeError:
			raise gdb.GdbError("no such type: {0}".format(dtype))
	
		found = False
		for field in typ.strip_typedefs().fields():
			if (field.name == dmemb):
				try:
					weenix.assert_type(field.type, _list_link_type)
				except gdb.GdbError as err:
					raise gdb.GdbError(
						"field '{0}' of type '{1}' has wrong type: {2}"
						.format(dmemb, dtype, str(err)))
				found = True
		if (not found):
			raise gdb.GdbError("'{0}' type does not contain field '{1}'"
							   .format(dtype, dmemb))

	value = name if isinstance(name, gdb.Value) else gdb.parse_and_eval(name)
	return List(value, dtype, dmemb)
