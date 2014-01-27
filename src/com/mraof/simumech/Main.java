package com.mraof.simumech;

import java.io.BufferedReader;
import java.io.IOException;
import java.io.InputStreamReader;

import com.mraof.simumech.irc.IRC;
import com.mraof.simumech.markov.MarkovChain;
import com.mraof.simumech.skype.SkypeBot;

public class Main 
{
	public static String clientName = "Simumech";
	public static String version = "0";

	public static String[] owners = {"Mraof"};

	public static void main(String args[])
	{
		MarkovChain markovChain = new MarkovChain();
		
		IRC irc = new IRC();
		boolean useSkype = false;
		SkypeBot skypeBot = new SkypeBot();

		BufferedReader bufferedReader = new BufferedReader(new InputStreamReader(System.in));
		String inputString;

		try {
			while((inputString = bufferedReader.readLine()) != null)
			{
				if(inputString.equalsIgnoreCase("QUIT"))
				{
					break;
				}
				System.out.println(markovChain.reply(inputString));
			}
		} catch (IOException e) {
			e.printStackTrace();
		}
		System.out.println("Input loop done");
		irc.quit();
		skypeBot.quit();
		
		try {
			Thread.sleep(5000);
		} catch (InterruptedException e) {e.printStackTrace();}
		
		System.out.println(Thread.getAllStackTraces().keySet());
	}
}
