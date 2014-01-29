package com.mraof.simumech;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.Hashtable;

public class IntMap<L, S>
{
	ArrayList<S> list = new ArrayList<S>();
	HashMap<L, Integer> map = new HashMap<L, Integer>();
	
	public S get(int index)
	{
		return list.get(index);
	}
	
	public Integer lookup(L key)
	{
		return map.get(key);
	}
	
	public Integer add(S value, L key)
	{
		if(map.get(value) != null)
			return map.get(value);
		list.add(value);
		map.put(key, list.size() - 1);
		return list.size() - 1;
	}
	
}
