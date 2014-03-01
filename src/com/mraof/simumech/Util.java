package com.mraof.simumech;

import java.util.ArrayList;

public class Util 
{
	public static ArrayList<String> split(String string, String splitter) {
		ArrayList<String> strings = new ArrayList<String>();
		for (int i = string.indexOf(splitter); i != -1; i = string.indexOf(splitter)) {
			if(i != 0 && !string.substring(0, i).equals(splitter))
				strings.add(string.substring(0, i));
			string = string.substring(i + splitter.length());
		}
		strings.add(string);
		return strings;
	}

	public static ArrayList<String> split(String string) 
	{
		return split(string, " ");
	}
	public static ArrayList<String> split(String string, String... splitters)
	{
		ArrayList<String> strings = new ArrayList<String>();
		ArrayList<String> oldStrings = new ArrayList<String>();
		oldStrings.add(string);
		for(String splitter : splitters)
		{
			strings = new ArrayList<String>();
			for(String currentString : oldStrings)
				strings.addAll(split(currentString, splitter));
			oldStrings = strings;
		}
		return strings;
	}

	public static String selectivelyLowerCase(String string)
	{
		if(string.toLowerCase().startsWith("http:") || string.toLowerCase().startsWith("https:"))
			return string;
		else 
			return string.toLowerCase();
	}
}
