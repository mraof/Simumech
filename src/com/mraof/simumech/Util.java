package com.mraof.simumech;

import java.util.ArrayList;

public class Util 
{
	public static ArrayList<String> split(String string, String splitter) {
		ArrayList<String> strings = new ArrayList<String>();
		for (int i = string.indexOf(splitter); i != -1; i = string.indexOf(splitter)) {
			strings.add(string.substring(0, i));
			string = string.substring(i+splitter.length());
		}
		if(strings.isEmpty())
			strings.add(string);
		return strings;
	}

	public static ArrayList<String> split(String string) 
	{
		return split(string, " ");
	}
}
