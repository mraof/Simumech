package com.mraof.simumech;

import java.io.BufferedReader;
import java.io.FileNotFoundException;
import java.io.FileReader;
import java.io.IOException;

public class Config 
{
	String filename;
	
	public Config(String filename)
	{
		this.filename = filename;
	}
	
	public void load()
	{
		FileReader fileReader = null;
		try {
			fileReader = new FileReader("config");
		} catch (FileNotFoundException e) {	e.printStackTrace(); }
		
		BufferedReader reader = new BufferedReader(fileReader);
		
		try {
			reader.close();
		} catch (IOException e) {
			e.printStackTrace();
		}
		
	}
}
